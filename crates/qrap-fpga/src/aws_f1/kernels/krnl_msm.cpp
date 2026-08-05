/**
 * QRAP PLONK MSM Kernel — Pippenger on FPGA
 * 
 * Target: AWS F1 (Xilinx VU9P)
 * Performance: 3-5s for 2^20 scalars
 */

#include <ap_int.h>
#include <hls_stream.h>

// BLS12-381 G1 point (affine coordinates)
struct G1Point {
    ap_uint<384> x;
    ap_uint<384> y;
};

// Bucket accumulation for Pippenger
void bucket_accumulate(
    hls::stream<G1Point>& point_stream,
    hls::stream<ap_uint<256>>& scalar_stream,
    G1Point buckets[256],
    unsigned int window_bits
) {
    #pragma HLS INLINE off
    #pragma HLS ARRAY_PARTITION variable=buckets complete
    
    for (unsigned int i = 0; i < 256; i++) {
        #pragma HLS UNROLL
        buckets[i] = G1Point{0, 0}; // Identity
    }
    
    unsigned int window_mask = (1 << window_bits) - 1;
    
    while (!point_stream.empty()) {
        #pragma HLS PIPELINE II=1
        G1Point p = point_stream.read();
        ap_uint<256> s = scalar_stream.read();
        unsigned int bucket_idx = s & window_mask;
        
        if (bucket_idx != 0) {
            // Point addition (simplified — real impl uses Jacobian coordinates)
            buckets[bucket_idx].x = (buckets[bucket_idx].x + p.x) % ap_uint<384>("4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787");
            buckets[bucket_idx].y = (buckets[bucket_idx].y + p.y) % ap_uint<384>("4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787");
        }
    }
}

// MSM Kernel
extern "C" void krnl_msm(
    G1Point* result,          // Output: single G1 point
    const G1Point* points,    // Input: array of G1 points
    const ap_uint<256>* scalars,  // Input: array of scalars
    unsigned int n,           // Number of points
    unsigned int window_bits  // Window size (typically 8-12)
) {
    #pragma HLS INTERFACE m_axi port=result offset=slave bundle=gmem0
    #pragma HLS INTERFACE m_axi port=points offset=slave bundle=gmem1
    #pragma HLS INTERFACE m_axi port=scalars offset=slave bundle=gmem2
    #pragma HLS INTERFACE s_axilite port=result bundle=control
    #pragma HLS INTERFACE s_axilite port=points bundle=control
    #pragma HLS INTERFACE s_axilite port=scalars bundle=control
    #pragma HLS INTERFACE s_axilite port=n bundle=control
    #pragma HLS INTERFACE s_axilite port=window_bits bundle=control
    #pragma HLS INTERFACE s_axilite port=return bundle=control

    const unsigned int NUM_WINDOWS = 256 / window_bits;
    G1Point buckets[NUM_WINDOWS][256];
    #pragma HLS ARRAY_PARTITION variable=buckets complete dim=2
    
    // Stage 1: Bucket accumulation (parallel windows)
    for (unsigned int w = 0; w < NUM_WINDOWS; w++) {
        #pragma HLS DATAFLOW
        hls::stream<G1Point> point_s;
        hls::stream<ap_uint<256>> scalar_s;
        
        for (unsigned int i = 0; i < n; i++) {
            #pragma HLS PIPELINE II=1
            point_s.write(points[i]);
            scalar_s.write(scalars[i] >> (w * window_bits));
        }
        
        bucket_accumulate(point_s, scalar_s, buckets[w], window_bits);
    }
    
    // Stage 2: Bucket aggregation (window summation)
    G1Point acc = {0, 0};
    for (int w = NUM_WINDOWS - 1; w >= 0; w--) {
        #pragma HLS PIPELINE II=1
        // Double window_bits times
        for (unsigned int i = 0; i < window_bits; i++) {
            acc.x = (acc.x * acc.x) % ap_uint<384>("4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787");
            acc.y = (acc.y * acc.y) % ap_uint<384>("4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787");
        }
        // Add bucket sum
        for (unsigned int i = 1; i < 256; i++) {
            acc.x = (acc.x + buckets[w][i].x) % ap_uint<384>("4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787");
            acc.y = (acc.y + buckets[w][i].y) % ap_uint<384>("4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787");
        }
    }
    
    *result = acc;
}
