This document describes the rationale behind having chosen Lua as the extension language for Blackjack.

# Why a scripting language in the first place?

The realization that Blackjack needs to be backed by a full-blown scripting language came after I considered the best way to implement several upcoming features: Dynamic selections and Mesh channels.

There will be documents decribing these features once they land, but without ellaborating much on what these features are just yet, I can say most of them rely on the idea of executing some custom behaviour *for every* element of the mesh (face, edge, vertex... but for simplicity this document will just call these: *per-vertex* operations).

This new kind of customization poses two new challenges:

- Users need a way to express this 'per vertex' logic. How will this representation look like? Taking a hint from other similar tools, the key seems to be in finding a balance between nodes and a textual language, managing to get the best of both worlds. 

- Regardless of the representation, this custom logic is user *code*. And as much as I'd like self-modifying Rust code, it is not a thing, so Blackjack is going to need some sort of interpreter to accept code modifications at runtime. Moreover, this interpreter is now going to be running some expressions for every vertex of a mesh, so it needs to be somewhat fast.

The thing is, the old implementation based on a half-baked VM (a.k.a. PolyASM) failed on both departments: It didn't have a textual representation and it was definitely not fast enough as an interpreter to handle per-vertex expressions. Fixing these two problems essentially boils down to making a programming language: frontend and backend, from scratch. As fun as that may sound, it is a considerable detour.

Another important aspect is user-generated content. The current version of Blackjack has too many missing features and most of them are missing nodes. The problem with the current approach is that every new node needs to be added manually by the developers. This goes against the spirit of it being a tool for tinkerers: Users need to be able to make their own tools without having to fill an issue and wait for the devs to do it.

The solution I see here is to move from a framework where nodes are the API, to a framework where there is a programming API, which nodes are using underneath. This is pretty much the definition of a scripting language.

So, from two different angles, we reach the same conclusion: Blackjack needs a scripting language. And in that scenario, keeping a separate interpreter around (i.e. what I used to call polyasm) is just extra maintenance burden. Instead, the graphs can be compiled to scripts, and those scripts can use the same programming API available to users.

# Then, why Lua?
I have spent a lot of time evaluating various options. I didn't start with Lua in mind, and I was a bit hesitant about choosing Lua for a Rust project. But after having looked everywhere, I believe it's the solution that gets me closer to my vision of what scripting / user code should look like for Blackjack, both in terms of UX and API. I won't say Lua fits *every* one of the project's needs, but it's by far the best combination of tradeoffs I could find.

I have compared Lua to several other alternatives. Here's a summary:

## Wasm
My first go-to solution for this was webassembly. Being fast, sandboxed and having several (but not *that* many I, as came to learn) languages supporting it, it seemed to be a perfect fit. The truth, though, is that it's still a bit early to build a plugin system with wasm. The moment you want to share anything non-trivial between host and guest (that is, something other than an integer), you're mostly in completely uncharted territory and having to invent your own ABI.

Proposals like Interface Types are *exactly* what Blackjack needs and are going to completely eliminate this issue once implemented. But implementations are still far away, with no clear roadmap I could find (could be months, could be years :shrug:). I'm aware of tools like wit-bindgen trying to bridge this gap, but after trying to build some minimal MVP with them, I realized they don't yet support Blackjack's use case well: In particular, support for sharing resource types between modules is not yet implemented (https://github.com/bytecodealliance/wit-bindgen/issues/120), and that is a major requirement for Blackjack.

Overall, my feeling for wasm is that it's *too early*, so I will wait until interface types are stable, then re-evaluate my choice.

## Rhai
Rhai's progress looks absolutely impressive, and scored 10 out of 10 on ease of integration with Rust.

But on the not-so-bright side, I benchmarked Rhai vs Lua for what I believe was a representative use case for Blackjack. When it comes to host (i.e. Rust) interoperability, both are pretty much tied. But when doing some non-trivial computations on the script side, Lua outperformed Rhai by a quite significant margin. I mainly attribute this to Lua(JIT) having a JIT compiler, which Rhai doesn't, so it wasn't too surprising.

My other issue with Rhai is adoption. I know it's a circular argument (it'll never get adoption if people don't adopt it!). But being pragmatic, Lua has lots of documentation, tutorials, and is well-known on the gamedev/modding/scripting community. Moreover, it has tooling: A language server, documentation generators, a (not great, but also not terrible) library ecosystem. There is a great chance Blackjack users already know Lua, whereas learning Rhai is going to be an extra requirement for pretty much anyone (even for its author!).

## Python
I took a quick look at Python, and PyO3 seems to be quite nice. But I had trouble finding documentation and I don't need to benchmark Python to know it's going to be a bit too slow for my taste unless I spend a lot of time building "native" abstractions similar to what pandas / numpy do to fill the data science niche.

Overall, it looked like it can be done and it may even be a great fit, but it also seems a bit too complex. I'd rather have something where the integration effort is not so high.

## Javascript
Javascript looked promising. V8 is known to be a very fast interpreter, as fast as an interpreted dynamic language can be. But unfortunately, the point I made for Python is doubly true here: I wouldn't even know where to start taming that V8 beast. I also want to stress that one of Blackjack's main use cases is game engine integration and people are not going to appreciate their games pulling all of V8 with them just for a few procedural meshes.

There are other more lightweight alternatives to V8, both in terms of ease-of-integration and memory footprint. But in that case I suspect JS would loose most of its competitive advantage to other runtimes. I haven't benchmarked this though, so I may be wrong there.

Finally, my last problem with a possible JS integration is the lack of operator overloading. This makes vector math in Javascript look terrible. And I expect users to do quite a bit of vector math in scripts.

## Mun
In a similar fashion to Rhai, Mun looked quite promising on a surface level. Perhaps even more so, because it takes performance very seriously and tackles the hot-reloading problem head-on unlike any other compiled language out there.

But after delving into it, I'm not even sure Mun is meant to be embedded into Rust projects like blackjack. Even if it is, it seemed to be too far from their focus. As it is today, it seems Mun's goal is to become a standalone programming language where Rust is just an implementation detail. Someone more knowledgeable about Mun may want to correct me about that though, that was just my impression from browsing the existing documentation and open issues.

In a similar vein to JS above, mun also leverages a heavy runtime for compilation: LLVM. Blackjack needs to compile code on-the-fly, and for Mun that (probably?) means having to ship LLVM with it. Just like V8, people are not going to want to ship a heavy runtime with their games.

## Lua
And last, but not least, we reach Lua. After having seen all the alternatives, Lua too has its pros and cons:

- When compared to wasm, it's slow. There's LuaJit, but the Jit seems to be unable to kick in when there's interaction with the host (Rust). So opportunities for running fast are limited to "pure" lua code. Less than ideal, but not a deal breaker: Most other runtimes I explored are also slower than wasm, and wasm can't be used until the interface types propsal is implemented, even past its MVP stage. This is not going to happen anytime soon.

- When compared to Rhai, integration with Rust is slightly more cumbersome. But not by far, and it's nothing a few macros can't fix. The [mlua](https://github.com/khvzak/mlua) crate is already making a great job about that.

- When compared to statically typed languages, there's several drawbacks in Lua. Not having the compiler help you debug basic code before it runs is certainly going to be an issue when people start developing non-trivial extensions. That's why I'm still planning to include a mechanism for native Rust extensions, even if that requires having to manually compile Blackjack. The scripting language should only be used for simple stuff where the rigidness of the Rust compiler is not needed, or where performance does not matter. If lack of static analysis becomes too much of an issue, there's the possibility of using [Luau](https://luau-lang.org/), which adds gradual typing. But it remains to be seen whether Luau as a whole fits blackjack well.

- When compared to Python and Javascript, it's slightly less well known and has substantially smaller ecosystem. I haven't benchmarked it, but performance may be better in JS runtimes like V8. Tooling is also inferior in many aspects (language server implementations for JS and Python are *far* better).

But at the end of the day, I am quite convinced with Lua because it feels it brings the right set of tradeoffs to the table:

- Okay performance: Not great when compared to Rust, but has a JIT and can run fast enough with a bit of tweaking.
- Lightweight: I intend to have a portion of Blackjack distributed with games (see #13), so this is paramount. And frankly, there's few runtimes more lightweight than Lua's.
- Well known: Again, not the best but far from the last. People know about Lua.
- Simplicity: We're heading into personal opinion territory, but I think Lua is a very simple language, in a good way. It gets a very small subset of fundamental primitives (function, table, metatable) and lets you do *a lot* with them. I like simple languages.
- I try to never argue about syntax when discussing programming languages, but Lua's syntax is _okay_ for newcomers. It's familiar to the C crowd, feels lean like Python's, but doesn't fall into the trap of significant whitespace and lets people use an auto-formatter instead.
- And last, but not least, I'm a Lisper at heart. I won't get into the benefits of Lisp here, but where Lua runs, there's [Fennel](https://github.com/bakpakin/Fennel), and I plan to eventually add support for it. This means Lisp enjoyers will get their dose in Blackjack without pushing a decision others my find `(odd)` and hurting adoption 🙂