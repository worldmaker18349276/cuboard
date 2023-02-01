# cuboard: Turn your smart cube into a keyboard

Turn your smart cube to type something, and reverse turns to fix mistypings. Now it only
supports GAN 356 i3, because I only have this one.

## How to use
It's still a prototype and can only be played in the command line. It is developed under
the ubuntu system with bluez (linux bluetooth stack). In theory, it should also support
other operating systems and bluetooth libraries. Note that it utilizes ANSI escape code to
produce stylized output.

Build from source:

```
git clone git@github.com:worldmaker18349276/cuboard.git
cd cuboard
cargo build --release
```

Run:

```
./target/release/cuboard
```

Pass the filename as an argument for the typing exercise:

```
./target/release/cuboard README.md
```

Note that line breaks should be done manually, otherwise the output will be messed up.

## How does it works
A keyboard has many keys, how to mimic a keyboard by turning only six sides? How to
differentiate between a reverse turn and a forward turn?

To solve the first question, a key is encoding into a sequence of turns, for example,
`F2U` indicates "K", `UR'` indicates "z", and `RU'` indicates "{". But now the second
question becomes tricky: "fix mistypings by reversing" means `F2U2R'RU'` should also
indicate "K" (since `UR'` is deleted by `RU'`), but not "Kz{". 

For this, the encoding should be based on paths of operation (the fundamental groupoid),
not actions, and each key is assigned to some kind of path.

### Path and canonical form
For a rubik's cube, some turns commute each others. For example, `LRL'` is equivalent to
`RLL'` since `L` and `R` commute. It is also equivalent to `R` since adjacent reversed
turns can be cancelled out. Drawing on the state space of a rubik's cube, those three
paths are homotopic in topology. For example, the algorithm `RUR'UR'U'R2U'R'UR'URU2` is
equivalent to `U [[U',R], [U,R']] 3U`, where `[A,B] = ABA'B'` is commutator. Note that
`U4` is not equivalent to identity operation in this sense.

To recognize which two operations are homotopic, one can _sort_ them into a canonical
form:

1. If two adjacent actions are commute, such as `L` and `R`, sort them in a specific
   order.
2. If two adjacent actions are reversed each other, such as `U` and `U'`, eliminate them.

Two operations are homotopic if and only if they have the same canonical form. The
encoding of keys is based on the canonical form of the input sequences, but here we modify
the first rule a little:

1. If two adjacent actions are commute, such as `L` and `R`, sort them in the order of
   last appearing.

With this rule, the order of commuting turns can be changed by hand, otherwise the
encoding will become harder. For example, if you misturned `LR` as `RL`, it can be fixed
by `RR'`, since `RLRR'` will be sorted into `LR`.

### Key encoding and memorizable layout
With the pseudo-canonical form, keys can be encoded into sequences of turns. The default
keymap encodes each key as two non-commuting turns, such as `UF`. There are 8 of such
operations with the same second symbol: `UF`, `RF`, `DF`, `LF`, `U'F`, `R'F`, `D'F`,
`L'F`. The direction of the first symbol should not be considered, otherwise one cannot
represent `LU'` followed by `UF` (`LU'UF` and `LF` are indistinguishable; it can be
alternated by `LU'2F` since `U'F` and `UF` are assigned to the same key).

By grouping with the second symbol, a group contians 4 keys, and there are 12 symbols:
`U`, `D`, `L`, `R`, `F`, `B`, `U'`, `D'`, `L'`, `R'`, `F'`, `B'`. Furthermore, the first
symbol can be replaced with a double turn. So there are 96 keys in total, which
corresponds to 95 printable characters (0x20 ~ 0x7E) + enter key (0x0A).

A part of table:

|      | `L` | `B` | `R` | `F` | `L2` | `B2` | `R2` | `F2` |
| ---- | --- | --- | --- | --- | ---- | ---- | ---- | ---- |
| `U`  | d   | u   | c   | k   | D    | u    | c    | k    |
| `U'` | (   | \[  | {   | <   | )    | ]    | }    | >    |
| `D`  | n   | x   | j   | i   | N    | X    | J    | I    |
| `D'` | \\  | +   | -   | *   | /    | =    | \|   | ^    |

Where the first cell of each column indicates the first symbol, and the first cell of each
row indicates the second symbol.

To visualize the keymap layout, imagine you're holding a white cube and placing each of
the 4 keys of the symbol on the edge of the face. For example, the key of `LU` (that is,
"k") should be placed on the left edge of the top face. In this way, there are four types
of rubik's cube skins: the first symbol is single turn or double turn; the second symbol
is clockwise or counterclockwise.

To remember them, read clockwise from the ULF corner (or DRB corner). For example, in the
single turn and clockwise skin, all six faces are assigned to 6 words: "verb", "duck",
"flow", "myth", "gasp" and "jinx". And the double turn variant correspond to uppercase
version. Two missing letters "z" and "q" are assigned to counterclockwise variants of "s"
and "p". In the counterclockwise skins, numbers 0~9 and whitespace/enter are assigned to
the left and right faces, note that whitespace and enter are assigned to `FR'` and `F2R'`,
which are heavily used; brackets are assigned to the top face; punctuations are assigned
to the front face; arithmetic symbols are assigned to the bottom face; other symbols are
assigned to the back face.

The full cheatsheet can be drawn as:

```
 first ||     double     |      single    |     single     |     double
second ||    clockwise   |    clockwise   |counterclockwise|counterclockwise
-------||----------------|----------------|----------------|----------------
   B   ||      VERB      |      verb      |      @$&`      |      #%~_
   U   ||      DUCK      |      duck      |      ([{<      |      )]}>
 L F R || MYTH FLOW GASP | myth flow gasp | 1234 '.:! 0⌴zq | 5678 ",;? 9↵ZQ
   D   ||      JINX      |      jinx      |      +-*/      |      =|^\

⌴: whitespace
↵: enter
```

## TODO
- [ ] customize keymap.
- [ ] remap orientation.
- [ ] shake to cancel.
- [ ] turn around to use another set of (action-based) keymap.
- [ ] mimic keyboard event.
- [ ] gui.
