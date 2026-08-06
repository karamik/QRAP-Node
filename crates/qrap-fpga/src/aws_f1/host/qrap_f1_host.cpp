#include <cstdint>
#include <cstddef>
#include <vector>
#include <string>
#include <stdexcept>
#include <CL/cl2.hpp>
#include "../kernels/qrap_kernels.h"

class QrapF1Host {
    cl::Context ctx;
    cl::CommandQueue q;
    cl::Program prog;
    cl::Kernel fe_mul_k, fe_add_k, ntt_k, msm_bucket_k;

    cl::Buffer buf_a, buf_b, buf_c;

public:
    int init(const char* xclbin_path) {
        std::vector<cl::Platform> platforms;
        cl::Platform::get(&platforms);
        if (platforms.empty()) return -1;

        std::vector<cl::Device> devices;
        platforms[0].getDevices(CL_DEVICE_TYPE_ACCELERATOR, &devices);
        if (devices.empty()) return -2;

        ctx = cl::Context(devices[0]);
        q = cl::CommandQueue(ctx, devices[0], CL_QUEUE_PROFILING_ENABLE);

        /* Load xclbin */
        std::ifstream bin_file(xclbin_path, std::ios::binary);
        if (!bin_file) return -3;
        std::vector<char> bin_buf(std::istreambuf_iterator<char>(bin_file), {});
        cl::Program::Binaries bins{{bin_buf.data(), bin_buf.size()}};
        prog = cl::Program(ctx, devices, bins);
        try {
            prog.build(devices);
        } catch (...) {
            return -4;
        }

        fe_mul_k = cl::Kernel(prog, "fe_mul_batch");
        fe_add_k = cl::Kernel(prog, "fe_add_batch");
        ntt_k = cl::Kernel(prog, "ntt_radix2");
        msm_bucket_k = cl::Kernel(prog, "msm_bucket_accum");
        return 0;
    }

    int fe_mul_batch(const qrap_fe256_t* a, const qrap_fe256_t* b,
                     qrap_fe256_t* c, uint32_t n) {
        size_t bytes = n * sizeof(qrap_fe256_t);
        buf_a = cl::Buffer(ctx, CL_MEM_READ_ONLY | CL_MEM_COPY_HOST_PTR, bytes, (void*)a);
        buf_b = cl::Buffer(ctx, CL_MEM_READ_ONLY | CL_MEM_COPY_HOST_PTR, bytes, (void*)b);
        buf_c = cl::Buffer(ctx, CL_MEM_WRITE_ONLY, bytes);

        fe_mul_k.setArg(0, buf_a);
        fe_mul_k.setArg(1, buf_b);
        fe_mul_k.setArg(2, buf_c);
        fe_mul_k.setArg(3, n);

        q.enqueueNDRangeKernel(fe_mul_k, cl::NullRange, cl::NDRange(n), cl::NullRange);
        q.enqueueReadBuffer(buf_c, CL_TRUE, 0, bytes, c);
        q.finish();
        return 0;
    }

    int ntt(qrap_fe256_t* inout, const qrap_fe256_t* twiddles, uint32_t log_n) {
        uint32_t n = 1u << log_n;
        size_t data_bytes = n * sizeof(qrap_fe256_t);
        size_t tw_bytes = (n/2) * sizeof(qrap_fe256_t);

        cl::Buffer buf_data(ctx, CL_MEM_READ_WRITE | CL_MEM_COPY_HOST_PTR, data_bytes, inout);
        cl::Buffer buf_tw(ctx, CL_MEM_READ_ONLY | CL_MEM_COPY_HOST_PTR, tw_bytes, (void*)twiddles);

        ntt_k.setArg(0, buf_data);
        ntt_k.setArg(1, buf_tw);
        ntt_k.setArg(2, log_n);

        for (uint32_t stage = 0; stage < log_n; stage++) {
            ntt_k.setArg(3, stage);
            q.enqueueNDRangeKernel(ntt_k, cl::NullRange, cl::NDRange(n/2), cl::NullRange);
        }
        q.enqueueReadBuffer(buf_data, CL_TRUE, 0, data_bytes, inout);
        q.finish();
        return 0;
    }
};

/* C API wrappers */
extern "C" {

void* qrap_f1_create() { return new QrapF1Host(); }
void qrap_f1_destroy(void* h) { delete (QrapF1Host*)h; }

int qrap_f1_init(void* h, const char* xclbin) {
    return ((QrapF1Host*)h)->init(xclbin);
}

int qrap_f1_fe_mul(void* h, const qrap_fe256_t* a, const qrap_fe256_t* b,
                   qrap_fe256_t* c, uint32_t n) {
    return ((QrapF1Host*)h)->fe_mul_batch(a, b, c, n);
}

int qrap_f1_ntt(void* h, qrap_fe256_t* inout, const qrap_fe256_t* tw, uint32_t log_n) {
    return ((QrapF1Host*)h)->ntt(inout, tw, log_n);
}

} /* extern "C" */

// Standalone test main (optional, for hw_emu/hw validation)
#ifdef QRAP_F1_STANDALONE
#include <cstdio>
#include <cstdlib>

int main(int argc, char** argv) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <xclbin_path>\n", argv[0]);
        return 1;
    }
    QrapF1Host host;
    int rc = host.init(argv[1]);
    if (rc != 0) {
        fprintf(stderr, "F1 init failed: %d\n", rc);
        return 1;
    }
    printf("AWS F1 initialized: %s\n", argv[1]);
    return 0;
}
#endif
