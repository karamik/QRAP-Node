# Vitis HLS Tcl script — krnl_msm
# Multi-Scalar Multiplication (Pippenger)

set src_dir "hls"
set build_dir "build"
set top_func "krnl_msm"

open_project -reset ${build_dir}/krnl_msm_prj
set_top ${top_func}
add_files ${src_dir}/krnl_msm.cpp

open_solution -reset "solution1"
set_part {xcvu9p-flgb2104-2-i}
create_clock -period 3.33 -name default

# MSM-specific optimizations
config_compile -pipeline_loops 8
config_dataflow -default_channel fifo -fifo_depth 32

csynth_design
export_design -format xo -output ${build_dir}/krnl_msm.xo

close_project
exit
