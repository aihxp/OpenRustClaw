#!/usr/bin/env python3
"""Script to generate protobuf Python code from proto definitions."""

import subprocess
import sys
from pathlib import Path


def generate_protobuf():
    """Generate Python code from protobuf definitions."""
    # Get the sidecar directory
    sidecar_dir = Path(__file__).parent.absolute()
    proto_dir = sidecar_dir.parent / "proto"
    src_dir = sidecar_dir / "src"
    proto_file = proto_dir / "orchestration.proto"

    if not proto_file.exists():
        print(f"Error: Proto file not found at {proto_file}")
        sys.exit(1)

    print(f"Generating protobuf Python code from {proto_file}")
    print(f"Output directory: {src_dir}")

    # Run protoc
    cmd = [
        sys.executable,
        "-m",
        "grpc_tools.protoc",
        f"-I{proto_dir}",
        f"--python_out={src_dir}",
        f"--grpc_python_out={src_dir}",
        str(proto_file),
    ]

    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        print(f"Error: {result.stderr}")
        sys.exit(1)

    print("Protobuf generation successful!")

    # Move generated files to proto package
    proto_package_dir = src_dir / "proto"
    proto_package_dir.mkdir(exist_ok=True)

    generated_files = [
        src_dir / "orchestration_pb2.py",
        src_dir / "orchestration_pb2_grpc.py",
    ]

    for file in generated_files:
        if file.exists():
            dest = proto_package_dir / file.name
            # Read, fix import, and write
            content = file.read_text()
            # Fix import
            content = content.replace(
                "import orchestration_pb2 as",
                "from proto import orchestration_pb2 as"
            )
            dest.write_text(content)
            file.unlink()  # Remove original
            print(f"Moved and fixed {file.name} -> {dest}")

    # Ensure __init__.py exists
    init_file = proto_package_dir / "__init__.py"
    if not init_file.exists():
        init_file.write_text("# Protobuf generated code package\n")
        print(f"Created {init_file}")

    print("Done!")


if __name__ == "__main__":
    generate_protobuf()
