#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "unistd.h"

char *tests[] = {
    // "mnt/test_mount", /* 1 */ 
    "brk",              /* 2 */ 
    "chdir",            /* 3 */ 
    "clone",            /* 4 */ 
    "close",            /* 5 */ 
    "dup",              /* 6 */ 
    "dup2",             /* 7 */ 
    "execve",           /* 8 */ 
    "exit",             /* 9 */ 
     "fork",            /* 10 */
     "fstat",           /* 11 */
     "getcwd",          /* 12 */
     "getdents",        /* 13 */
     "getpid",          /* 14 */
     "getppid",         /* 15 */
     "gettimeofday",    /* 16 */
    //  "mkdir_",          /* 17 */
     "mmap",            /* 18 */
     "mount",           /* 19 */
     "munmap",          /* 20 */
     "open",            /* 21 */
    //  "openat",          /* 22 */
    //  "pipe",            /* 23 */
     "read",            /* 24 */
     "sleep",           /* 25 */
     "test_echo",       /* 26 */
     "times",           /* 27 */
     "umount",          /* 28 */
     "uname",           /* 29 */
     "unlink",          /* 30 */
     "wait",            /* 31 */
     "waitpid",         /* 32 */
     "write",           /* 33 */
     "yield",           /* 34 */
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
