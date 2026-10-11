"""Build and run C23/C++26 ABI checks and examples against an existing cargo build."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--library-dir", default="target/debug")
    parser.add_argument("--cc", default="gcc")
    parser.add_argument("--cxx", default="g++")
    parser.add_argument("--cpp-standard", default="c++26")
    parser.add_argument("--ssgf", action="store_true", help="check a library built with --features ssgf")
    args = parser.parse_args()
    library = (ROOT / args.library_dir).resolve()
    output = ROOT / "build/native"
    output.mkdir(parents=True, exist_ok=True)
    windows = sys.platform == "win32"
    if windows:
        dll = library / "cani.dll"
        if not dll.exists():
            parser.error("build cani first with cargo build")
        link = [str(dll)]
        shutil.copy2(dll, output / "cani.dll")
    else:
        link = ["-L", str(library), "-lcani", "-Wl,-rpath," + str(library)]
    sources = [
        (args.cc, "c23", "tests/native.c", "test-c"),
        (args.cxx, args.cpp_standard, "tests/native.cpp", "test-cpp"),
        (args.cc, "c23", "examples/c23/main.c", "example-c"),
        (args.cxx, args.cpp_standard, "examples/cpp26/main.cpp", "example-cpp"),
        (args.cc, "c23", "examples/bots/c23.c", "bot-c"),
        (args.cxx, args.cpp_standard, "examples/bots/cpp26.cpp", "bot-cpp"),
    ]
    if args.ssgf:
        sources.extend([(args.cc,"c23","tests/ssgf_native.c","ssgf-c"),
                        (args.cxx,args.cpp_standard,"tests/ssgf_native.cpp","ssgf-cpp")])
    environment = os.environ.copy()
    if windows:
        # Include the compiler directory for GCC's C++ runtime DLLs.
        compiler = shutil.which(args.cxx)
        if compiler:
            environment["PATH"] = str(Path(compiler).parent) + os.pathsep + environment.get("PATH", "")
    for compiler, standard, source, name in sources:
        executable = output / (name + (".exe" if windows else ""))
        defines = ["-DCANI_WITH_SSGF"] if args.ssgf else []
        subprocess.run([compiler, "-std=" + standard, "-Wall", "-Wextra", "-Werror", *defines,
                        "-I", str(ROOT / "include"), str(ROOT / source), *link,
                        "-o", str(executable)], check=True, cwd=ROOT)
        if not name.startswith("bot-"):
            subprocess.run([str(executable)], check=True, cwd=ROOT, env=environment)


if __name__ == "__main__":
    main()
