#!/bin/bash
set -e

# QRAP FPGA Build Script
# Usage: ./build.sh [sw_emu|hw_emu|hw]

TARGET=${1:-hw}
PLATFORM=${PLATFORM:-xilinx_aws-vu9p-f1_shell-v04261818_201920_3}

echo "========================================"
echo "  QRAP PLONK FPGA Build"
echo "  Target: $TARGET"
echo "  Platform: $PLATFORM"
echo "========================================"

# Check environment
if [ -z "$XILINX_VITIS" ]; then
    echo "ERROR: Source Vitis environment first:"
    echo "  source /opt/xilinx/Vitis/2024.1/settings64.sh"
    exit 1
fi

if [ -z "$XILINX_XRT" ]; then
    echo "ERROR: Source XRT environment first:"
    echo "  source /opt/xilinx/xrt/setup.sh"
    exit 1
fi

echo "Vitis: $XILINX_VITIS"
echo "XRT:   $XILINX_XRT"

# Clean previous build
if [ "$TARGET" = "hw" ]; then
    echo "Cleaning previous build..."
    make clean
fi

# Build
if [ "$TARGET" = "sw_emu" ]; then
    make emu
else
    make all
fi

echo ""
echo "========================================"
echo "  Build Complete"
echo "========================================"
if [ "$TARGET" = "hw" ]; then
    echo "Output: build/qrap_plonk.xclbin"
    echo ""
    echo "Next steps:"
    echo "1. Create AFI: make afi S3_BUCKET=your-bucket"
    echo "2. Load AFI:   sudo fpga-load-local-image -S 0 -I agfi-xxxxxxxx"
    echo "3. Test:       ./target/release/qrap-node fpga-bench -b aws-f1"
fi
