##
## 用于在没有 Fat32 的时候将测试程序链接到内核中跑测试
##

import std/[strformat, strutils]
import std/os

const
  syscallTestPathPrefix = "../../misc/syscalltests/"
  syscallTestPath = "syscalltests/"
  buildinAsmPath = "../os/src/buildin_app.S"
  ignoredTests = "ignored_tests"

proc genAppAsmBlock(id: int, name = "", path: string): string =
  result & fmt"""
.section .data
.globl app_{id}_start
.globl app_{id}_end
.align 3
        app_{id}_start:
                .incbin "{syscallTestPathPrefix}{path}"
        app_{id}_end:

"""

proc readInExcludedTests(): seq[string] =
  result = open(ignoredTests, fmRead).readAll().split('\n')

let excludedTests = readInExcludedTests()

proc testname(path: string): string =
  path.split('/')[^1]

proc addTestAppFromDirRecursion(f: File, dirname: string, i: var int) =
  for (kind, path) in walkDir(dirname):
    if kind == pcDir: addTestAppFromDirRecursion(f, path, i)
    if excludedTests.contains(path.testname): continue

    f.write(genAppAsmBlock(id = i, path = path))

    inc i


proc main() =
  let f = open(buildinAsmPath, fmWrite)
  var i = 0

  addTestAppFromDirRecursion(f, syscallTestPath, i)

  echo "All syscall tests added."
  
when isMainModule:
  main()