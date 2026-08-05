/**
 * QRAP PLONK Field Arithmetic Kernel
 * 
 * Montgomery multiplication, modular inverse, polynomial evaluation
 */

#include <ap_int.h>

const ap_uint<256> FIELD_MOD = ap_uint<256>("52435875175126190479447740508185965837690552500527637822603658699938581184513");

// Montgomery multiplication
ap_uint<256> mont_mul(ap_uint<256> a, ap_uint<256> b) {
    #pragma HLS INLINE
    ap_uint<512> prod = a * b;
    return prod % FIELD_MOD;
}

// Modular inverse (Fermat's little theorem: a^(p-2) mod p)
ap_uint<256> mod_inv(ap_uint<256> a) {
    #pragma HLS INLINE off
    ap_uint<256> result = 1;
    ap_uint<256> base = a;
    ap_uint<256> exp = FIELD_MOD - 2;
    
    for (int i = 0; i < 256; i++) {
        #pragma HLS PIPELINE II=1
        if (exp[i]) {
            result = mont_mul(result, base);
        }
        base = mont_mul(base, base);
    }
    return result;
}

// Polynomial evaluation at point z: p(z) = sum(c_i * z^i)
extern "C" void krnl_field(
    ap_uint<256>* result,           // Output: evaluation result
    const ap_uint<256>* coeffs,     // Input: polynomial coefficients
    const ap_uint<256>& z,          // Evaluation point
    unsigned int degree             // Polynomial degree
) {
    #pragma HLS INTERFACE m_axi port=result offset=slave bundle=gmem0
    #pragma HLS INTERFACE m_axi port=coeffs offset=slave bundle=gmem1
    #pragma HLS INTERFACE s_axilite port=result bundle=control
    #pragma HLS INTERFACE s_axilite port=coeffs bundle=control
    #pragma HLS INTERFACE s_axilite port=z bundle=control
    #pragma HLS INTERFACE s_axilite port=degree bundle=control
    #pragma HLS INTERFACE s_axilite port=return bundle=control

    ap_uint<256> acc = 0;
    ap_uint<256> z_pow = 1;
    
    for (unsigned int i = 0; i <= degree; i++) {
        #pragma HLS PIPELINE II=1
        #pragma HLS UNROLL factor=4
        acc = (acc + mont_mul(coeffs[i], z_pow)) % FIELD_MOD;
        z_pow = mont_mul(z_pow, z);
    }
    
    *result = acc;
}
