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
    parser.add_argument("--c-standard", default="c23")
    parser.add_argument("--ssgf", action="store_true", help="check a library built with --features ssgf")
    parser.add_argument("--sanitizers", choices=["address", "address,undefined"], help="instrument native callers; combine with a sanitizer-built Rust library for core coverage")
    parser.add_argument("--msvc", action="store_true", help="use cl from a Visual Studio developer shell (C latest/C++ latest)")
    args = parser.parse_args()
    if args.msvc and args.sanitizers == "address,undefined":
        parser.error("MSVC supports address sanitizer only")
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
        (args.cc, args.c_standard, "tests/native.c", "test-c"),
        (args.cc, args.c_standard, "tests/transport_native.c", "transport-c"),
        (args.cc, args.c_standard, "tests/extensions.c", "extensions-c"),
        (args.cxx, args.cpp_standard, "tests/extensions.cpp", "extensions-cpp"),
        (args.cxx, args.cpp_standard, "tests/native.cpp", "test-cpp"),
        (args.cc, args.c_standard, "examples/c23/main.c", "example-c"),
        (args.cxx, args.cpp_standard, "examples/cpp26/main.cpp", "example-cpp"),
        (args.cc, args.c_standard, "examples/bots/c23.c", "bot-c"),
        (args.cxx, args.cpp_standard, "examples/bots/cpp26.cpp", "bot-cpp"),
    ]
    if args.ssgf:
        sources.extend([(args.cc,args.c_standard,"tests/ssgf_native.c","ssgf-c"),
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
        if args.msvc:
            command = ["cl", "/nologo", "/std:clatest" if source.endswith(".c") else "/std:c++latest", "/W3", "/WX", "/EHsc", "/MD", "/Zi",
                       *(["/fsanitize=address"] if args.sanitizers else []),
                       *(["/DCANI_WITH_SSGF"] if args.ssgf else []), "/I" + str(ROOT / "include"),
                       str(ROOT / source), str(library / "cani.dll.lib"), "/Fe:" + str(executable),
                       "/Fo:" + str(output / (name + ".obj")), "/Fd:" + str(output / (name + ".pdb"))]
        else:
            instrument = ["-fsanitize=" + args.sanitizers, "-fno-omit-frame-pointer", "-g"] if args.sanitizers else []
            command = [compiler, "-std=" + standard, "-Wall", "-Wextra", "-Werror", *instrument, *defines,
                       "-I", str(ROOT / "include"), str(ROOT / source), *link, "-o", str(executable)]
        subprocess.run(command, check=True, cwd=ROOT)
        if not name.startswith("bot-"):
            subprocess.run([str(executable)], check=True, cwd=ROOT, env=environment)


if __name__ == "__main__":
    main()
