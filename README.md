# epic_bevy_remote_reflection

Implement `bevy_reflect`'s `Reflect` trait for remote type, bypassing the orphan
rule by editing trait-object's virtual table at runtime.

# Unsafe, do not use for production! (winked)

I only created this for fun.
Rust doesn't have a guarantee that the underlying virtual table memory layout will
stays the same between Rust releases. It's a part of the compiler, not the language.
So be aware of changes and untested version!

Tested versions:
- `1.80`

https://geo-ant.github.io/blog/2023/rust-dyn-trait-objects-fat-pointers/ 
