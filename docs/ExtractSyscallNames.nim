import std/[strutils, strformat]

let f = open("oscomp_syscalls.md", fmRead)

var syscalls = newSeq[(string, string)]()

while not f.endOfFile:
  let line = f.readLine
  if line.startsWith("###"):
    let 
      syscallInfoTuple = line.substr(line.find("define ") + 7).split(' ')
      syscallInfo = (syscallInfoTuple[0].toUpper, syscallInfoTuple[1])
    syscalls.add(syscallInfo)

template rustConstSyscall(syscallName, syscallId: string): string =
  fmt"""pub const {syscallName}: usize = {syscallId};"""

for (syscallName, syscallId) in syscalls:
  echo rustConstSyscall(syscallName, syscallId)

echo "total syscalls: " & $syscalls.len
