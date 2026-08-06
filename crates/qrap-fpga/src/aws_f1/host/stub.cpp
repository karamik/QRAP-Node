#include <cstdint>
#include <cstdlib>

extern "C" {

void* qrap_f1_create() { return nullptr; }
void qrap_f1_destroy(void*) {}
int qrap_f1_init(void*, const char*) { return 0; }
int qrap_f1_fe_mul(void*, const void*, const void*, void*, uint32_t) { return -1; }
int qrap_f1_ntt(void*, void*, const void*, uint32_t) { return -1; }

}
