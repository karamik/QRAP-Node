# Vitis HLS Tcl script — krnl_ntt
# Number Theoretic Transform for BLS12-381 scalar field

set src_dir "hls"
set build_dir "build"
set top_func "krnl_ntt"

open_project -reset ${build_dir}/krnl_ntt_prj
set_top ${top_func}
add_files ${src_dir}/krnl_ntt.cpp

open_solution -reset "solution1"
set_part {xcvu9p-flgb2104-2-i}
create_clock -period 3.33 -name default
# 300MHz target

# Directives for NTT optimization
config_compile -pipeline_loops 16
config_dataflow -default_channel fifo -fifo_depth 16

# Interface directives (applied in source via pragmas)
# #pragma HLS INTERFACE m_axi port=inout offset=slave bundle=gmem0

csynth_design
export_design -format xo -output ${build_dir}/krnl_ntt.xo

close_project
exit
