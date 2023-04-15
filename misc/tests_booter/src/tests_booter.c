#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "unistd.h"

char *tests[] = {
    "brk",       "chdir",   "clone",        "close",  "dup",    "dup2",
    "execve",    "exit",    "fork",         "fstat",  "getcwd", "getdents",
    "getpid",    "getppid", "gettimeofday", "mkdir_", "mmap",   "mount",
    "munmap",    "open",    "openat",       "pipe",   "read",   "sleep",
    "test_echo", "times",   "umount",       "uname",  "unlink", "wait",
    "waitpid",   "write",   "yield",
};

#define NTESTS 35

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
