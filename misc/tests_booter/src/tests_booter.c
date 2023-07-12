#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "unistd.h"

char *argv_sh[] = {"./busybox", "sh", 0};
char *argv_lua[] = {"./lua", 0, 0};

int main() {
    int npid = fork();
    assert(npid >= 0);

    int child_return;
    if (npid == 0) {
      execve("./busybox", argv_sh, NULL);
      /* execve("./lua", argv_lua, NULL); */
    } else {
      child_return = -1;
      waitpid(npid, &child_return, 0);
    }

  return 0;
}
