### Why can `cd` not be an external program?

When an operating system runs an external program (like `ls`, `grep`, or `wc`), it creates a completely new **child process** for that program to run in. This child process receives its own independent environment block and a copy of the current working directory from the shell. 

If `cd` were written as an external program:
1. The shell would spawn a child process for `cd`.
2. The `cd` child process would successfully change *its own* current working directory.
3. The `cd` program would finish executing and its process would terminate.
4. Control would return to the parent shell process, which remains completely unchanged in its original directory.

Because a child process cannot modify the working directory of its parent process, changing directories **must** happen directly inside the shell process itself. This is why `cd` has to be implemented as an internal **shell builtin** rather than an external executable file.


### Author
- Priyanshu Bikash Mohapatra
