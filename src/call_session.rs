// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! One explicitly confirmed call. Audio and pending text are memory bounded.
use crate::call_audio::AudioChunk;
use crate::call_stt::{Configuration, SttWorker, SubmitOutcome};
use std::collections::BTreeMap;

const RATE: usize = 16_000;
const FRAME: usize = 1_600;
const LIMIT: usize = 6 * RATE;
const SILENCE: usize = RATE * 6 / 10;

#[derive(Default)]
struct Segmenter { samples: Vec<i16>, lead: Vec<i16>, quiet: usize, voiced: usize, continuation: bool }
struct Segment { samples: Vec<i16>, overlaps_previous: bool }
impl Segmenter {
    fn push(&mut self, input: &[i16]) -> Vec<Segment> {
        let mut output = Vec::new();
        for frame in input.chunks(FRAME) {
            let energy = frame.iter().map(|v| i64::from(*v).pow(2) as u64).sum::<u64>();
            let speech = energy > 180 * 180 * frame.len() as u64;
            if self.samples.is_empty() && !speech {
                self.lead.extend_from_slice(frame);
                let trim = self.lead.len().saturating_sub(RATE / 5);
                self.lead.drain(..trim);
                continue;
            }
            if self.samples.is_empty() { self.samples.append(&mut self.lead); }
            self.samples.extend_from_slice(frame);
            if speech { self.voiced += frame.len(); self.quiet = 0; } else { self.quiet += frame.len(); }
            let full = self.samples.len() >= LIMIT;
            if full || self.quiet >= SILENCE {
                let mut samples = std::mem::take(&mut self.samples);
                let continuation = self.continuation;
                self.continuation = false;
                if self.voiced >= RATE / 10 {
                    // Forced cuts overlap by 0.5s; text merging removes repeated words.
                    if full && self.quiet < SILENCE {
                        self.samples.extend_from_slice(&samples[samples.len().saturating_sub(RATE / 2)..]);
                        self.continuation = true;
                    }
                    // Preserve short acknowledgements such as "sì" after a pause.
                    samples.resize(samples.len().max(RATE), 0);
                    output.push(Segment { samples, overlaps_previous: continuation });
                }
                self.voiced = if self.samples.is_empty() { 0 } else { RATE / 10 };
                self.quiet = 0;
            }
        }
        output
    }
}

fn tail(text: &str, bytes: usize) -> String {
    let mut start = text.len().saturating_sub(bytes);
    while !text.is_char_boundary(start) { start += 1; }
    text[start..].to_string()
}
/// Add only the new suffix when consecutive audio segments overlap.
fn merge(previous: &mut String, text: &str, overlaps: bool) -> String {
    let next: Vec<_> = text.split_whitespace().collect();
    let old: Vec<_> = previous.split_whitespace().rev().take(20).collect::<Vec<_>>().into_iter().rev().collect();
    let overlap = if overlaps { (1..=old.len().min(next.len())).rev().find(|&n|
        old[old.len()-n..].iter().zip(&next[..n]).all(|(a,b)| a.to_lowercase() == b.to_lowercase())).unwrap_or(0) } else { 0 };
    let suffix = next[overlap..].join(" ");
    if !suffix.is_empty() {
        if !previous.is_empty() { previous.push('\n'); }
        previous.push_str(&suffix);
        *previous = tail(previous, 16_384);
    }
    suffix
}

pub struct CallSession {
    pub generation: u64,
    stt: SttWorker,
    segmenter: Segmenter,
    pub transcript: String,
    pub notice: Option<String>,
    pub suggestion: String,
    pub context: String,
    coach: Option<crate::call_coach::Coach>,
    next_segment: u64,
    last_result: u64,
    overlaps: BTreeMap<u64, bool>,
}
impl CallSession {
    pub fn new(generation: u64, config: Configuration, context: String, with_ai: bool) -> Result<Self, String> {
        if context.len() > 2048 { return Err("Obiettivo della call troppo lungo".into()); }
        let stt = SttWorker::new(config)?;
        let coach = if with_ai { Some(crate::call_coach::Coach::start(generation, context.clone())?) } else { None };
        Ok(Self { generation, stt, segmenter: Segmenter::default(),
            transcript: String::new(), notice: None, suggestion: String::new(), context, coach,
            next_segment: 0, last_result: 0, overlaps: BTreeMap::new() })
    }
    pub fn push_audio(&mut self, chunk: AudioChunk) -> Result<(), String> {
        if chunk.generation != self.generation { return Ok(()); }
        if chunk.sample_rate != RATE as u32 || chunk.pcm.len() > RATE { return Err("Formato o dimensione audio inattesi".into()); }
        for segment in self.segmenter.push(&chunk.pcm) {
            self.next_segment = self.next_segment.checked_add(1).ok_or("Limite dei segmenti audio raggiunto")?;
            self.overlaps.insert(self.next_segment, segment.overlaps_previous);
            while self.overlaps.len() > 32 { self.overlaps.pop_first(); }
            if self.stt.submit_segment(self.generation, self.next_segment, segment.samples)? == SubmitOutcome::ReplacedQueued {
                self.notice = Some("Trascrizione in ritardo: un segmento in attesa è stato sostituito.".into());
            }
        }
        Ok(())
    }
    pub fn tick(&mut self) -> Result<(), String> {
        if let Some(result) = self.stt.poll() {
            if result.generation != self.generation { return Ok(()); }
            if result.dropped_results > 0 { self.notice = Some("Un risultato non letto è stato sostituito: la trascrizione può avere una lacuna.".into()); }
            match result.text {
                Ok(text) => {
                    let overlap = self.last_result.checked_add(1) == Some(result.segment_id)
                        && self.overlaps.get(&result.segment_id).copied().unwrap_or(false);
                    let fresh = merge(&mut self.transcript, &text, overlap);
                    self.last_result = result.segment_id;
                    self.overlaps.retain(|id, _| *id > result.segment_id);
                    if !fresh.is_empty() {
                        if let Some(coach) = &self.coach {
                            if let Err(error) = coach.publish(&fresh) { self.notice = Some(error); }
                        }
                    }
                }
                Err(error) => return Err(error),
            }
        }
        if let Some(coach) = &self.coach {
            if let Some(suggestion) = coach.take_suggestion() { self.suggestion = suggestion; }
        }
        Ok(())
    }
    pub fn ai_enabled(&self) -> bool { self.coach.is_some() }
    pub fn stop(&mut self) {
        if let Some(coach) = &mut self.coach { coach.stop(); }
        self.stt.stop(); self.segmenter = Segmenter::default(); self.overlaps.clear();
    }
}
impl Drop for CallSession { fn drop(&mut self) { self.stop(); } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_has_no_segments_and_speech_flushes_after_pause() {
        let mut s = Segmenter::default();
        for _ in 0..100 { assert!(s.push(&[0; FRAME]).is_empty()); }
        assert!(s.samples.is_empty()); assert!(s.lead.len() <= RATE / 5);
        for _ in 0..12 { assert!(s.push(&[500; FRAME]).is_empty()); }
        let mut output = Vec::new();
        for _ in 0..6 { output.extend(s.push(&[0; FRAME])); }
        assert_eq!(output.len(),1); assert!(output[0].samples.len() <= LIMIT); assert!(s.samples.is_empty());
    }
    #[test]
    fn continuous_speech_is_bounded_and_overlaps_at_cuts() {
        let mut s = Segmenter::default(); let mut count=0;
        for _ in 0..300 {
            for segment in s.push(&[700; FRAME]) { assert!(segment.samples.len() <= LIMIT + FRAME); count+=1; }
            assert!(s.samples.len() < LIMIT); assert!(s.lead.len() <= RATE/5);
        }
        assert!(count >= 4);
    }
    #[test]
    fn overlap_merging_keeps_new_speech_and_bounds_utf8() {
        let mut transcript = "Possiamo fissare la riunione".to_string();
        assert_eq!(merge(&mut transcript,"la riunione domani mattina",true),"domani mattina");
        assert_eq!(merge(&mut transcript,"domani mattina",true),"");
        assert_eq!(merge(&mut transcript,"domani mattina",false),"domani mattina");
        merge(&mut transcript,&"è ".repeat(10000),false);
        assert!(transcript.len() <= 16384); assert!(std::str::from_utf8(transcript.as_bytes()).is_ok());
    }
    #[test]
    fn short_acknowledgements_are_padded_instead_of_discarded() {
        let mut s = Segmenter::default();
        assert!(s.push(&[500; FRAME]).is_empty());
        let mut output = Vec::new();
        for _ in 0..6 { output.extend(s.push(&[0; FRAME])); }
        assert_eq!(output.len(),1); assert_eq!(output[0].samples.len(),RATE);
        assert!(!output[0].overlaps_previous);
    }
}
