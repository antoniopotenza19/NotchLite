// Original NotchLite module. SPDX-License-Identifier: Apache-2.0
//! One explicitly confirmed call. Audio and pending text are memory bounded.
use crate::call_audio::AudioChunk;
use crate::call_stt::{Configuration, SttWorker, SubmitOutcome};
use std::collections::BTreeMap;

const RATE: usize = 16_000;
const FRAME: usize = 1_600;
const LIMIT: usize = 6 * RATE;
const SILENCE: usize = RATE * 6 / 10;

fn above_noise_floor(frame:&[i16])->bool {
    frame.iter().map(|v|i64::from(*v).pow(2)as u64).sum::<u64>()>180*180*frame.len()as u64
}

#[derive(Default)]
struct Segmenter { samples: Vec<i16>, lead: Vec<i16>, quiet: usize, voiced: usize, continuation: bool }
struct Segment { samples: Vec<i16>, overlaps_previous: bool }
impl Segmenter {
    fn push(&mut self, input: &[i16]) -> Vec<Segment> {
        let mut output = Vec::new();
        for frame in input.chunks(FRAME) {
            let speech = above_noise_floor(frame);
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
                        // A silent retained tail shares timing, but no spoken
                        // words. Do not let it erase a genuine repeated reply.
                        self.continuation = self.samples.chunks(FRAME).any(above_noise_floor);
                    }
                    // Preserve short acknowledgements such as "sì" after a pause.
                    samples.resize(samples.len().max(RATE), 0);
                    output.push(Segment { samples, overlaps_previous: continuation });
                }
                // Retained overlap has already been transcribed. It must not
                // count as new speech, especially if a forced cut ended in
                // silence; otherwise the following pause sends an overlap-only
                // or entirely silent segment back to Whisper.
                self.voiced = 0;
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
    profile_label: String,
    next_segment: u64,
    last_result: u64,
    overlaps: BTreeMap<u64, bool>,
}
impl CallSession {
    pub fn new(generation: u64, config: Configuration, context: String, with_ai: bool) -> Result<Self, String> {
        if context.len() > 2048 { return Err("Obiettivo della call troppo lungo".into()); }
        let notice=if config.vad_model.is_none(){Some("Riconoscimento senza rilevatore vocale: aggiorna il profilo con Setup-CallTranscription.ps1 per ridurre il testo spurio nelle pause.".into())}else{None};
        let profile_label=config.profile_label();
        let stt = SttWorker::new(config)?;
        let coach = if with_ai { Some(crate::call_coach::Coach::start(generation, context.clone())?) } else { None };
        Ok(Self { generation, stt, segmenter: Segmenter::default(),
            transcript: String::new(), notice, suggestion: String::new(), context, coach, profile_label,
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
    pub fn profile_label(&self) -> &str { &self.profile_label }
    pub fn ai_status(&self) -> Option<String> {self.coach.as_ref().map(|coach|coach.status())}
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
    #[test]
    fn a_pause_after_a_forced_cut_does_not_transcribe_silence_or_overlap_alone() {
        for voiced_frames in [55,60] {
            let mut segmenter=Segmenter::default();let mut output=Vec::new();
            for _ in 0..voiced_frames {output.extend(segmenter.push(&[500;FRAME]));}
            for _ in voiced_frames..60 {output.extend(segmenter.push(&[0;FRAME]));}
            assert_eq!(output.len(),1,"One six-second segment was emitted");
            for _ in 0..10 {assert!(segmenter.push(&[0;FRAME]).is_empty(),"Old overlap is not fresh speech");}
            assert!(segmenter.samples.is_empty());
            // A genuinely new short acknowledgement still reaches transcription.
            assert!(segmenter.push(&[500;FRAME]).is_empty());
            let mut fresh=Vec::new();for _ in 0..6 {fresh.extend(segmenter.push(&[0;FRAME]));}
            assert_eq!(fresh.len(),1);assert!(!fresh[0].overlaps_previous);
        }
    }
    #[test]
    fn silent_overlap_does_not_erase_a_genuinely_repeated_no() {
        let mut segmenter=Segmenter::default();let mut first=Vec::new();
        for _ in 0..55 {first.extend(segmenter.push(&[500;FRAME]));}
        for _ in 0..5 {first.extend(segmenter.push(&[0;FRAME]));}
        assert_eq!(first.len(),1);assert!(segmenter.samples.iter().all(|sample|*sample==0));
        // New speech arrives immediately after the forced cut, before the old
        // quiet interval has had another chance to flush the retained tail.
        assert!(segmenter.push(&[500;FRAME]).is_empty());
        let mut next=Vec::new();for _ in 0..6 {next.extend(segmenter.push(&[0;FRAME]));}
        assert_eq!(next.len(),1);assert!(!next[0].overlaps_previous);
        let mut transcript="No".to_string();
        assert_eq!(merge(&mut transcript,"no",next[0].overlaps_previous),"no");
        assert_eq!(transcript,"No\nno");
    }
}
