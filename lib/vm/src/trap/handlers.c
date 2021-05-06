// This file contains partial code from other sources.
// Attributions: https://github.com/wasmerio/wasmer/blob/master/ATTRIBUTIONS.md

#include <setjmp.h>
       #include <stdio.h>

// Note that `sigsetjmp` and `siglongjmp` are used here where possible
// to explicitly pass a 0 argument to `sigsetjmp` that we don't need
// to preserve the process signal mask. This should make this call a
// bit faster because it doesn't need to touch the kernel signal
// handling routines.
#ifdef TARGET_OS_WINDOWS
#define platform_setjmp(buf) setjmp(buf)
#define platform_longjmp(buf, arg) longjmp(buf, arg)
#define platform_jmp_buf jmp_buf
#else
#define platform_setjmp(buf) sigsetjmp(buf, 0)
#define platform_longjmp(buf, arg) siglongjmp(buf, arg)
#define platform_jmp_buf sigjmp_buf
#endif

int register_setjmp(
    void **buf_storage,
    void (*body)(void*),
    void *payload
) {
  platform_jmp_buf buf;

  if (platform_setjmp(buf) != 0) {
    return 0;
  }

  printf("Setjmp 0\n");

  *buf_storage = &buf;
  body(payload);
  printf("Setjmp 1\n");

  return 1;
}

void unwind(void *jump_buf) {
  printf("DOING UNWIND\n");
  platform_jmp_buf *buf = (platform_jmp_buf*) jump_buf;
  printf("DOING LONGJMP\n");
  platform_longjmp(*buf, 1);
}