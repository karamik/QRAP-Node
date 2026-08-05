# Vitis HLS Tcl script — krnl_field
# Field arithmetic: Montgomery multiplication, modular inverse

set src_dir "hls"
set build_dir "build"
set top_func "krnl_field"

open_project -reset ${build_dir}/krnl_field_prj
set_top ${top_func}
add_files ${src_dir}/krnl_field.cpp

open_solution -reset "solution1"
set_part {xcvu9p-flgb2104-2-i}
create_clock -period 3.33 -name default

# Field ops: lower II for multiplication
config_compile -pipeline_loops 4

csynth_design
export_design -format xo -output ${build_dir}/krnl_field.xo

close_project
exit
