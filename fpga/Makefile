# QRAP PLONK FPGA Build System
# Target: AWS F1 (Xilinx VU9P)
# Requires: Vitis 2024.1, XRT, AWS FPGA Developer AMI

PLATFORM := xilinx_aws-vu9p-f1_shell-v04261818_201920_3
PART := xcvu9p-flgb2104-2-i
VITIS_DIR := /opt/xilinx/Vitis/2024.1
XILINX_XRT := /opt/xilinx/xrt

# Kernels
KERNELS := krnl_ntt krnl_msm krnl_field
XOS := $(addprefix build/,$(addsuffix .xo,$(KERNELS)))
XCLBIN := build/qrap_plonk.xclbin

# Build directories
BUILD_DIR := build
SRC_DIR := hls
SCRIPT_DIR := scripts

.PHONY: all clean check-env kernels link xclbin emu hw

all: check-env $(XCLBIN)

check-env:
	@echo "Checking Xilinx environment..."
	@if [ ! -d "$(VITIS_DIR)" ]; then \
		echo "ERROR: Vitis not found at $(VITIS_DIR)"; \
		echo "Install: https://www.xilinx.com/support/download/index.html"; \
		exit 1; \
	fi
	@if [ ! -d "$(XILINX_XRT)" ]; then \
		echo "ERROR: XRT not found at $(XILINX_XRT)"; \
		echo "Install: https://github.com/Xilinx/XRT"; \
		exit 1; \
	fi
	@if [ -z "$(AWS_PLATFORM)" ]; then \
		echo "WARNING: AWS_PLATFORM not set. Using default: $(PLATFORM)"; \
	fi
	@echo "Environment OK"
	@echo "  Vitis: $(VITIS_DIR)"
	@echo "  XRT:   $(XILINX_XRT)"
	@echo "  Part:  $(PART)"

# Compile each kernel to .xo
kernels: $(XOS)

build/krnl_ntt.xo: $(SRC_DIR)/krnl_ntt.cpp $(SCRIPT_DIR)/krnl_ntt.tcl
	@echo "=== Synthesizing krnl_ntt ==="
	mkdir -p $(BUILD_DIR)
	vitis_hls $(SCRIPT_DIR)/krnl_ntt.tcl -l $(BUILD_DIR)/krnl_ntt.log

build/krnl_msm.xo: $(SRC_DIR)/krnl_msm.cpp $(SCRIPT_DIR)/krnl_msm.tcl
	@echo "=== Synthesizing krnl_msm ==="
	mkdir -p $(BUILD_DIR)
	vitis_hls $(SCRIPT_DIR)/krnl_msm.tcl -l $(BUILD_DIR)/krnl_msm.log

build/krnl_field.xo: $(SRC_DIR)/krnl_field.cpp $(SCRIPT_DIR)/krnl_field.tcl
	@echo "=== Synthesizing krnl_field ==="
	mkdir -p $(BUILD_DIR)
	vitis_hls $(SCRIPT_DIR)/krnl_field.tcl -l $(BUILD_DIR)/krnl_field.log

# Link kernels to .xclbin
link: $(XOS)
	@echo "=== Linking to $(XCLBIN) ==="
	v++ -l -t hw \
		--platform $(PLATFORM) \
		--connectivity.nk krnl_ntt:1:krnl_ntt_1 \
		--connectivity.nk krnl_msm:1:krnl_msm_1 \
		--connectivity.nk krnl_field:1:krnl_field_1 \
		--config constraints/link.cfg \
		-o $(XCLBIN) \
		$(XOS)

$(XCLBIN): link

# Emulation targets
emu:
	v++ -c -t sw_emu --platform $(PLATFORM) -k krnl_ntt -o build/krnl_ntt_emu.xo $(SRC_DIR)/krnl_ntt.cpp
	v++ -c -t sw_emu --platform $(PLATFORM) -k krnl_msm -o build/krnl_msm_emu.xo $(SRC_DIR)/krnl_msm.cpp
	v++ -c -t sw_emu --platform $(PLATFORM) -k krnl_field -o build/krnl_field_emu.xo $(SRC_DIR)/krnl_field.cpp
	v++ -l -t sw_emu --platform $(PLATFORM) -o build/qrap_plonk_emu.xclbin build/*_emu.xo

hw: all

clean:
	rm -rf $(BUILD_DIR) _x .Xil *.log *.jou

# AWS AFI creation
afi: $(XCLBIN)
	@echo "=== Creating AWS FPGA Image ==="
	aws ec2 create-fpga-image \
		--input-storage-location Bucket=$(S3_BUCKET),Key=qrap_plonk.xclbin \
		--logs-storage-location Bucket=$(S3_BUCKET),Key=logs \
		--name qrap-plonk-$(shell date +%Y%m%d) \
		--description "QRAP PLONK prover v0.2.0"

help:
	@echo "QRAP PLONK FPGA Build System"
	@echo ""
	@echo "Targets:"
	@echo "  make check-env  - Verify Xilinx environment"
	@echo "  make kernels    - Compile HLS kernels to .xo"
	@echo "  make link       - Link kernels to .xclbin"
	@echo "  make all        - Full build (kernels + link)"
	@echo "  make emu        - Software emulation build"
	@echo "  make hw         - Hardware build (same as all)"
	@echo "  make afi        - Create AWS FPGA Image"
	@echo "  make clean      - Remove build artifacts"
	@echo ""
	@echo "Environment variables:"
	@echo "  PLATFORM        - Xilinx platform (default: $(PLATFORM))"
	@echo "  S3_BUCKET       - AWS S3 bucket for AFI upload"
