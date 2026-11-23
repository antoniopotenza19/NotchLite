[CmdletBinding()]
param(
    [ValidateNotNullOrEmpty()]
    [string]$GraphifyPython = 'python'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not (Get-Command $GraphifyPython -ErrorAction SilentlyContinue)) {
    throw 'Python non trovato. Indica con -GraphifyPython un interprete che contiene Graphify. Nessuna installazione viene eseguita.'
}

# Only the local AST API is used. Do not replace it with the headless "extract"
# CLI, which may choose an LLM backend from the environment.
$pythonCode = @'
from pathlib import Path
from collections import Counter
import hashlib
import importlib.metadata
import json
import os
import sys

try:
    from graphify.detect import detect
    from graphify.extract import collect_files, extract
    from graphify.build import build_from_json
    from graphify.cluster import cluster, score_all
    from graphify.analyze import god_nodes, surprising_connections, suggest_questions
    from graphify.report import generate
    from graphify.export import to_json
except ImportError as exc:
    raise SystemExit(
        "Graphify non disponibile in questo interprete. Usa -GraphifyPython "
        "con un ambiente gia presente oppure continua la lettura diretta dei "
        "sorgenti. Nessun pacchetto e stato installato. Dettaglio: " + str(exc)
    )

root = Path(sys.argv[1]).resolve()
os.chdir(root)
if not Path("src").is_dir():
    raise SystemExit("Cartella src non trovata nel progetto.")
out = Path("graphify-out")
files = sorted(path for path in collect_files(Path("src")) if path.suffix == ".rs")
if not files:
    raise SystemExit("Nessun sorgente Rust trovato in src/.")
before = {path.as_posix(): hashlib.sha256(path.read_bytes()).hexdigest() for path in files}
detection = detect(Path("src"), follow_symlinks=False, google_workspace=False)
extraction = extract(files, cache_root=out / "cache", parallel=False)
after = {path.as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
         for path in collect_files(Path("src")) if path.suffix == ".rs"}
if before != after:
    raise SystemExit("I sorgenti sono cambiati durante la lettura: ripetere la mappa.")
if not extraction.get("nodes"):
    raise SystemExit("Estrazione vuota: il grafo precedente non viene sostituito.")

graph = build_from_json(extraction, directed=True)
communities = cluster(graph)
cohesion = score_all(graph, communities)
labels = {}
for group, nodes in communities.items():
    counts = Counter(Path(graph.nodes[node].get("source_file") or "unknown").stem for node in nodes)
    labels[group] = counts.most_common(1)[0][0]
report = generate(
    graph, communities, cohesion, labels, god_nodes(graph),
    surprising_connections(graph, communities), detection,
    {"input": 0, "output": 0}, "src",
    suggested_questions=suggest_questions(graph, communities, labels),
)
limits = (
    "# Current code map\n\n"
    "Local AST only. No application code, LLM backend or network command was run "
    "by this script. Source hashes are in SOURCE_SNAPSHOT.json. The map may miss "
    "dynamic dispatch, COM/FFI and runtime paths; it is not proof of correctness "
    "or safety. Check the hashes before treating it as current. Generated "
    "artifacts may be replaced on the next refresh; keep historical reviews "
    "under docs/review instead.\n\n"
)
try:
    version = importlib.metadata.version("graphifyy")
except importlib.metadata.PackageNotFoundError:
    version = "unknown"
snapshot = {
    "source_root": "src",
    "tool": {
        "distribution": "graphifyy",
        "version": version,
        "repository": "https://github.com/safishamsi/graphify",
        "mode": "local AST only",
    },
    "source_sha256": before,
    "file_count": len(files),
    "node_count": graph.number_of_nodes(),
    "edge_count": graph.number_of_edges(),
    "community_count": len(communities),
    "directed": True,
    "additional_llm_tokens": {"input": 0, "output": 0},
}
out.mkdir(exist_ok=True)
# This directory contains replaceable generated artifacts, not historic reports.
if not to_json(graph, communities, str(out / "graph.json"), force=True):
    raise SystemExit("Graphify non ha scritto il grafo.")
(out / "GRAPH_REPORT.md").write_text(limits + report, encoding="utf-8")
(out / "SOURCE_SNAPSHOT.json").write_text(
    json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
)
print(json.dumps({
    "output": "graphify-out",
    "files": len(files),
    "nodes": graph.number_of_nodes(),
    "edges": graph.number_of_edges(),
    "graphify_version": version,
    "additional_llm_tokens": 0,
}, ensure_ascii=False))
'@

& $GraphifyPython -X utf8 -c $pythonCode $projectRoot
if ($LASTEXITCODE -ne 0) {
    throw "Mappa non aggiornata (uscita $LASTEXITCODE). Nessuna installazione automatica."
}
