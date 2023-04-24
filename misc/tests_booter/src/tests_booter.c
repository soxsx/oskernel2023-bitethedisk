#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "unistd.h"

char *tests[] = {
    "brk",          /* 1 */
    "chdir",        /* 2 */
    "clone",        /* 3 */
    "close",        /* 4 */
    "dup",          /* 5 */
    "dup2",         /* 6 */
    "execve",       /* 7 */
    "exit",         /* 8 */
    "fork",         /* 9 */
    "fstat",        /* 10 */
    "getcwd",       /* 11 */
    "getdents",     /* 12 */
    "getpid",       /* 13 */
    "getppid",      /* 14 */
    "gettimeofday", /* 15 */
    "mkdir_",       /* 16 */
    "mmap",         /* 17 */
    "mount",        /* 18 */
    "munmap",       /* 19 */
    "open",         /* 20 */
    "openat",       /* 21 */
    // "pipe",             /* 22 */
    "read",      /* 23 */
    "sleep",     /* 24 */
    "test_echo", /* 25 */
    "times",     /* 26 */
    "umount",    /* 27 */
    "uname",     /* 28 */
    "unlink",    /* 29 */
    "wait",      /* 30 */
    "waitpid",   /* 31 */
    "write",     /* 32 */
    "yield",     /* 33 */
};

#define NTESTS (sizeof(tests) / sizeof(tests[0]))

int main() {
  for (int i = 0; i < NTESTS; i++) {
    int npid = fork();
    assert(npid >= 0);

    int child_return;
    if (npid == 0) {
      exec(tests[i]);
    } else {
      child_return = -1;
      waitpid(npid, &child_return, 0);
    }
  }

  return 0;
}
