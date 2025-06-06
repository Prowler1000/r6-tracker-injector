# R6 Tracker Injector
R6 Tracker has data and I want it.

R6 Tracker has data on users that it doesn't reveal to the client, and I'm interested in reading that data, specifically the usernames and possibly platforms of other players.

The idea, currently, is to inject a DLL into a running Overwolf instance, and scan through the memory, looking for a specific byte sequence that preceeds the data of interest. Ideally I'd just hook into the functions responsible for writting that memory but I'm not incredibly familiar with reverse engineering yet, so I'm having some difficulty figuring out what that function is.

I originally designed this in C++, implementing injection, ipc, etc. all manually. I switched to Rust because I was tired of having issues parsing UTF-16, mostly because of the design of the library I was using, and I felt error handling was a lot easier in Rust, on top of just being a nicer language in general.

## Todo
* ### Break apart the `client` crate
  * The client crate was originally designed to be "the" crate where "everything" was done. It was meant to hold both the code used by the DLL and the code used by the main app to communicate with the DLL. At some point during development though, I decided I wanted more control outside, in the root package (on top of switching from a virtual Workspace to.. whatever this is now), so things got a little messed up. Basically I just need to refactor, heavily.
* ### Improve `Logger` implementation and usage
  * For some reason, creating a dedicated logging crate was an after thought. (Not really a mystery, I know *why* I did it, it's just a stupid reason) As a result, its implementation and usage feel clunky, though that may just be me.
* ### Improve comments
  * My God, do you not remember the number of times you've cursed other OSS for poor documentation and comments only useful if you understand the entire codebase? So where the heck are *your* comments? If you wanna judge others for something, you probably shouldn't do the same thing yourself. (I mean, you shouldn't judge others in general, but still)
* ### Improve memory walking / Expand heap walking
  * Currently, if we want to actually find the data we're interested in, we have to walk the entire range of memory manually, which is unbelievably slow. This is a result of .NET doing its own allocations for heaps. I should properly understand how memory is allocated on Windows first (for instance, why do Windows heaps even exist if programs can allocate memory not in a Windows heap?) but you'll likely have to use something like `SOS.dll` to figure out where the heaps are stored as you have confirmed they are *not* at the same virtual address every run, and even if they were, that might change in the future. A possibility is *storing* where the heap is after a walk, but if you don't understand .NET memory management completely, it could just randomly not work and you'll waste a lot of time figuring out why.

This README is more for my own sanity. I'll probably drop this and come back to it after a while, so this serves as a reminder of how things started, what I needed to work on, and why some things are the way they are.