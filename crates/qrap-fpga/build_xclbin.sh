#!/usr/bin/env bash
set -euo pipefail

# QRAP FPGA Build Script for AWS F1 VU9P
# Usage: ./build_xclbin.sh [hw_emu|hw]

TARGET="${1:-hw_emu}"
PLATFORM="xilinx_aws-vu9p-f1_shell-v04261818_201920_2"
PLATFORM_PATH="/opt/xilinx/platforms/${PLATFORM}/${PLATFORM}.xpfm"

# Check Vitis
if ! command -v v++ &> /dev/null; then
    echo "ERROR: Vitis not found. Source /opt/xilinx/Vitis/2023.2/settings64.sh"
    exit 1
fi

# Check platform
if [ ! -f "$PLATFORM_PATH" ]; then
    echo "ERROR: Platform not found: $PLATFORM_PATH"
    echo "Install AWS FPGA developer kit: https://github.com/aws/aws-fpga"
    exit 1
fi

BUILD_DIR="build"
XCLBIN_DIR="xclbin"
mkdir -p "$BUILD_DIR" "$XCLBIN_DIR"

VPP_FLAGS="-t $TARGET --platform $PLATFORM_PATH -g -O3 --kernel_frequency 300"

echo "=== Building QRAP Kernels for AWS F1 ($TARGET) ==="

# Compile each kernel
for kernel in field ntt msm; do
    echo "[v++] Compiling $kernel.cl → $kernel.xo"
    v++ $VPP_FLAGS -c \
        -k "${kernel}_batch" \
        -o "$BUILD_DIR/${kernel}.xo" \
        "src/aws_f1/opencl/${kernel}.cl"
done

# Link
echo "[v++] Linking kernels → qrap_kernels.xclbin"
v++ $VPP_FLAGS -l \
    -o "$XCLBIN_DIR/qrap_kernels.xclbin" \
    "$BUILD_DIR"/field.xo \
    "$BUILD_DIR"/ntt.xo \
    "$BUILD_DIR"/msm.xo

echo "=== Build complete ==="
echo "XCLBIN: $XCLBIN_DIR/qrap_kernels.xclbin"
echo ""
echo "To run hardware emulation:"
echo "  export XCL_EMULATION_MODE=hw_emu"
echo "  ./build/qrap_f1_host $XCLBIN_DIR/qrap_kernels.xclbin"
echo ""
echo "To build for real FPGA (4-6 hours):"
echo "  ./build_xclbin.sh hw"
