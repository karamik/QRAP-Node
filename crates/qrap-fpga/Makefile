# QRAP FPGA — AWS F1 VU9P Build System
# Requires: Vitis 2023.2+, XRT, OpenCL headers

# Platform (AWS F1 VU9P)
DEVICE ?= xilinx_aws-vu9p-f1_shell-v04261818_201920_2
PLATFORM ?= /opt/xilinx/platforms/$(DEVICE)/$(DEVICE).xpfm

# Directories
SRC_DIR    = src/aws_f1
HOST_DIR   = $(SRC_DIR)/host
OCL_DIR    = $(SRC_DIR)/opencl
KRNL_DIR   = $(SRC_DIR)/kernels
BUILD_DIR  = build
XCLBIN_DIR = xclbin

# Kernels
KERNELS    = field ntt msm
XO_FILES   = $(patsubst %,$(BUILD_DIR)/%.xo,$(KERNELS))

# Host
HOST_SRC   = $(HOST_DIR)/qrap_f1_host.cpp
HOST_EXE   = $(BUILD_DIR)/qrap_f1_host

# Compiler flags
VPP        = v++
VPP_FLAGS  = -t hw_emu --platform $(PLATFORM) -g -O3 \
             --kernel_frequency 300 \
             --max_memory_ports field:4 \
             --sp field_1.m00_axi:HBM[0] \
             --sp field_1.m01_axi:HBM[1]

HOST_CXX   = g++
HOST_FLAGS = -std=c++14 -O2 -I$(KRNL_DIR) \
             -I/opt/xilinx/xrt/include \
             -I/usr/include/xrt \
             -L/opt/xilinx/xrt/lib -lOpenCL -lpthread -lrt -lstdc++

# Targets
.PHONY: all host xclbin clean emu hw

all: host xclbin

# Create dirs
$(BUILD_DIR) $(XCLBIN_DIR):
	mkdir -p $@

# Compile kernels to .xo
$(BUILD_DIR)/%.xo: $(OCL_DIR)/%.cl | $(BUILD_DIR)
	$(VPP) $(VPP_FLAGS) -c -k $*_batch -o $@ $<

# Link .xo → .xclbin
$(XCLBIN_DIR)/qrap_kernels.xclbin: $(XO_FILES) | $(XCLBIN_DIR)
	$(VPP) $(VPP_FLAGS) -l -o $@ $(XO_FILES)

# Build host executable
$(HOST_EXE): $(HOST_SRC) | $(BUILD_DIR)
	$(HOST_CXX) $(HOST_FLAGS) -o $@ $<

host: $(HOST_EXE)
xclbin: $(XCLBIN_DIR)/qrap_kernels.xclbin

# Hardware build (synthesis + implementation, ~4-6 hours)
hw: VPP_FLAGS += -t hw
hw: xclbin

# Hardware emulation (fast, for debug)
emu: VPP_FLAGS += -t hw_emu
emu: all

# Clean
clean:
	rm -rf $(BUILD_DIR) $(XCLBIN_DIR) *.log *.jou .Xil

# Run hw_emu
run_emu: emu
	export XCL_EMULATION_MODE=hw_emu && ./$(HOST_EXE) $(XCLBIN_DIR)/qrap_kernels.xclbin
