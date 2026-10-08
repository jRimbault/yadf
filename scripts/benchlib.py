"""Helpers shared by scripts/bench.py and scripts/bench-versions.py.

Not a script itself: both importers run under `uv run --script`, which puts
this directory first on sys.path, and declare its dependencies in their own
inline metadata.
"""

import argparse
import re
import shlex
import shutil
import subprocess
import sys
from collections.abc import Callable, Sequence
from datetime import UTC, datetime
from pathlib import Path
from typing import Literal, NoReturn

from pydantic import BaseModel, ConfigDict, ValidationError
from rich.console import Console
from rich.markup import escape
from rich.table import Table

SCRIPTS_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPTS_DIR.parent
CACHE_DIR = REPO_ROOT / ".bench-cache"
GEN_CORPUS = SCRIPTS_DIR / "gen-corpus.py"

DROP_CACHES = "sync && echo 3 | sudo tee /proc/sys/vm/drop_caches >/dev/null"

stdout = Console(highlight=False)
stderr = Console(stderr=True, highlight=False)


class Model(BaseModel):
    model_config = ConfigDict(frozen=True)


class Stat(Model):
    mean: float
    # null when hyperfine only got a single run out of a command
    stddev: float | None = None
    median: float
    min: float
    max: float


class Summary(Model):
    time_wall_clock: Stat
    time_user: Stat
    time_system: Stat
    # omitted when the platform cannot measure it
    memory_peak_resident: Stat | None = None


class Result(Model):
    name: str
    summary: Summary


class Export(Model):
    """hyperfine's JSON export, schema version 2 (hyperfine >= 2.0)."""

    results: tuple[Result, ...]


class Manifest(Model):
    """The part of gen-corpus.py's manifest.json the benchmarks check against."""

    duplicate_group_count: int


class Fatal(Exception):
    """An error worth reporting to the user without a traceback."""


def step(message: str) -> None:
    stderr.print(f"[bold blue]==>[/] {escape(message)}", soft_wrap=True)


def warn(message: str) -> None:
    stderr.print(f"[bold yellow]warning:[/] {escape(message)}", soft_wrap=True)


def entrypoint(main: Callable[[], None]) -> NoReturn:
    """Runs main, turning the expected failures into a message and exit code."""
    try:
        main()
    except Fatal as error:
        stderr.print(f"[bold red]error:[/] {escape(str(error))}", soft_wrap=True)
        sys.exit(1)
    except subprocess.CalledProcessError as error:
        command = shlex.join(str(arg) for arg in error.cmd)
        stderr.print(
            f"[bold red]error:[/] `{escape(command)}` exited with {error.returncode}"
        )
        sys.exit(1)
    except KeyboardInterrupt:
        sys.exit(130)
    sys.exit(0)


def run(*command: str | Path, cwd: Path | None = None) -> None:
    subprocess.run([str(arg) for arg in command], cwd=cwd, check=True)


def output(*command: str | Path, cwd: Path | None = None) -> str:
    return subprocess.run(
        [str(arg) for arg in command],
        cwd=cwd,
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def which(tool: str) -> Path | None:
    """Locates a tool in PATH, seeing through mise shims.

    A mise shim picks the version to run from the mise config of its working
    directory, but the timed programs run in the results directory, outside
    the scope of scripts/mise.toml, where the shims fail. Resolve them up front
    to the binary pinned there.
    """
    found = shutil.which(tool)
    if found is None:
        return None
    path = Path(found)
    if path.parent.name != "shims" or shutil.which("mise") is None:
        return path
    resolved = subprocess.run(
        ["mise", "which", tool],
        cwd=SCRIPTS_DIR,
        capture_output=True,
        text=True,
        check=False,
    )
    if resolved.returncode != 0:
        return None
    return Path(resolved.stdout.strip())


def require_tools(*tools: str) -> None:
    missing = [tool for tool in tools if which(tool) is None]
    if missing:
        raise Fatal(f"required but not found in PATH: {' '.join(missing)}")


def hyperfine_binary() -> Path:
    binary = which("hyperfine")
    if binary is None:
        raise Fatal("required but not found in PATH: hyperfine")
    return binary


def require_hyperfine_2() -> None:
    """The results are read from hyperfine's JSON export, whose schema changed in 2.0."""
    version = output(hyperfine_binary(), "--version").strip()
    match = re.match(r"hyperfine (\d+)\.", version)
    if match is None or int(match[1]) < 2:
        raise Fatal(f"hyperfine >= 2.0 is required, found {version}")


def require_passwordless_sudo() -> None:
    probe = subprocess.run(["sudo", "-n", "true"], capture_output=True, check=False)
    if probe.returncode != 0:
        raise Fatal("--cold requires passwordless sudo to drop the page cache")


def ensure_corpus(corpus: Path | None, *gen_args: str) -> Path:
    """Returns the corpus to scan, generating the default one on first use."""
    if corpus is None:
        corpus = CACHE_DIR / "corpus"
        if not (corpus / "manifest.json").is_file():
            step(f"generating default corpus at {corpus}")
            run(sys.executable, GEN_CORPUS, "--out", corpus, *gen_args)
    if not (corpus / "manifest.json").is_file():
        raise Fatal(f"corpus manifest not found at {corpus / 'manifest.json'}")
    return corpus


def load_manifest(corpus: Path) -> Manifest:
    path = corpus / "manifest.json"
    try:
        return Manifest.model_validate_json(path.read_bytes())
    except ValidationError as error:
        raise Fatal(f"invalid corpus manifest {path}:\n{error}") from error


def new_results_dir() -> Path:
    path = REPO_ROOT / "bench-results" / datetime.now(UTC).strftime("%Y%m%dT%H%M%S")
    path.mkdir(parents=True)
    return path


def add_run_count_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--warm-up",
        type=int,
        metavar="N",
        help="hyperfine warm-up runs per command (default 3 warm, 0 cold)",
    )
    parser.add_argument(
        "--min-runs",
        type=int,
        metavar="N",
        help="minimum hyperfine runs per command (default 10 warm, 5 cold)",
    )


def run_count_options(
    cold: bool, warm_up: int | None, min_runs: int | None
) -> list[str]:
    """hyperfine's --warmup and --min-runs, defaulting by cache state."""
    if warm_up is None:
        warm_up = 0 if cold else 3
    if min_runs is None:
        min_runs = 5 if cold else 10
    return ["--warmup", str(warm_up), "--min-runs", str(min_runs)]


def hyperfine(
    commands: Sequence[tuple[str, Sequence[str | Path]]],
    options: Sequence[str],
    results_dir: Path,
    cwd: Path | None = None,
) -> Export:
    """Times (name, argv) commands, and returns the parsed JSON export."""
    json_path = results_dir / "hyperfine.json"
    args: list[str] = [
        str(hyperfine_binary()),
        *options,
        "--export-json",
        str(json_path),
        "--export-markdown",
        str(results_dir / "hyperfine.md"),
    ]
    for name, _ in commands:
        args += ["--command-name", name]
    # hyperfine 2 runs commands without a shell, splitting them shell-style.
    args += [shlex.join(str(arg) for arg in argv) for _, argv in commands]
    run(*args, cwd=cwd)
    return Export.model_validate_json(json_path.read_bytes())


def plus_minus(stat: Stat, scale: float, digits: int) -> str:
    mean = f"{stat.mean * scale:.{digits}f}"
    if stat.stddev is None:
        return mean
    return f"{mean} ± {stat.stddev * scale:.{digits}f}"


def peak_rss_mib(summary: Summary) -> str:
    if summary.memory_peak_resident is None:
        return "n/a"
    return f"{summary.memory_peak_resident.median / 2**20:.1f}"


type Justify = Literal["left", "right"]


def report(
    columns: Sequence[tuple[str, Justify]], rows: Sequence[Sequence[str]], path: Path
) -> None:
    """Writes the rows as a markdown table to path, and prints them to stdout.

    hyperfine's own markdown export only shows the wall clock time, without the
    CPU time or the peak RSS.
    """
    rules = {"left": ":---", "right": "---:"}
    lines = [
        "| " + " | ".join(header for header, _ in columns) + " |",
        "| " + " | ".join(rules[justify] for _, justify in columns) + " |",
        *("| " + " | ".join(row) + " |" for row in rows),
    ]
    path.write_text("\n".join(lines) + "\n")

    table = Table()
    for header, justify in columns:
        table.add_column(escape(header), justify=justify)
    for row in rows:
        table.add_row(*(rich_cell(cell) for cell in row))
    stdout.print(table)


def rich_cell(markdown: str) -> str:
    """Translates the little markdown the tables use (code, bold) to rich markup."""
    text = escape(markdown.replace("`", ""))
    if text.startswith("**") and text.endswith("**"):
        return f"[bold]{text[2:-2]}[/]"
    return text
