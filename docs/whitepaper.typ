#import "@preview/fletcher:0.5.8" as fletcher: diagram, node, edge

#set page(paper: "a4")
#set heading(numbering: "1.")

#show link: set text(fill: blue, weight: 700)
#show link: underline

#set document(
  title: [OnlyFriends],
)

#place(
  top + center,
  float: true,
  scope: "parent",
  title(),
)

#align(center)[_Integritetsbyrån_]

#align(center)[*Abstract*]

OnlyFriends is a social media platform for staying connected with your real-life friends. It aims to be privacy friendly through e2e encryption and minimization of metadata leaks to third parties.

#outline()

= Terminology

- *App* An implementation of OnlyFriends.
- *User* An individual, described by a set of _device chains_.
- *Device* One app instance, belonging to one specific _user_.
- *Sibling* A _device_ belonging to the same _user_.
- *Device Chain* A linked list of `Entry`s, each pointing to an optional `Body`, expressing the state of a single _device_.
- *Entry* A cryptographically signed data type that is the building block of the _device graph_.

= Device Chains

A device is described only by a device chain. The device chain is an append only linked list, where each entry is cryptographically signed using ed25519ph #cite(<rfc8032>). Everything a user does is included in the device chain: Every post, every comment, every friend.

== Types

`Entry` is the root type of the device chain. Postcard is used for serialization.

```rust
type EntryId = Sha512<Entry>;

struct Entry {
  /// ID of the previous [`Entry`]
  prev: EntryId,
  /// Kind, ID, and length of the [`Body`] of this entry, if any.
  body: Option<(BodyContentKind, BodyId, u64)>,
  /// Longest seen sibling device chain. Used as a vector clock.
  siblings_len: Map<DeviceId, u64>,
}

type BodyId = Sha512<Body>;

struct Body {
  content: BodyContent,
}

enum BodyContent {
  Post { /* ... */ }
  Reaction { /* ... */ }
  // etc
}

/// The Device ID is the Entry ID of the device chain root node.
type DeviceId = EntryId;
```

The ID types are SHA512 hashes of the identified value.
Each `Entry` references its own `Body` by ID, as well as the previous `Entry`. This creates a linked list of entries -- the device chain, which facilitates a common view from any given device. More details in @sync.

#align(center)[#diagram(
  spacing: (2.2cm, 1.0cm),
  node-stroke: 1pt,
  node-fill: white,
  edge-stroke: 1pt,

  // Entrys
  node((0,0), [`Entry` 0], shape: circle, radius: 7mm),
  edge("<|-"),
  node((1,0), [`Entry` 1], shape: circle, radius: 7mm),
  edge("<|-"),
  node((2,0), [`Entry` 2], shape: circle, radius: 7mm),
  edge("<|-"),
  node((3,0), [`Entry` 4], shape: circle, radius: 7mm),
  edge("<|-"),
  node((4.0,0.0), [...etc], stroke: none),

  // Bodys
  edge((0,0), (0,1), "-|>"),
  node((0,1), [`Body`], shape: circle, radius: 6mm, fill: blue.lighten(70%)),
  edge((1,0), (1,1), "-|>"),
  node((1,1), [`Body`], shape: circle, radius: 6mm, fill: blue.lighten(70%)),
  edge((2,0), (2,1), "-|>"),
  node((2,1), [`Body`], shape: circle, radius: 6mm, fill: blue.lighten(70%)),
  edge((3,0), (3,1), "-|>"),
  node((3,1), [`Body`], shape: circle, radius: 6mm, fill: blue.lighten(70%)),
)]

`Body`s and `Entrys` serve two different purposes:
- An `Entry` is small, signed, with an upper bound on size. It's used to quickly synchronize the shape of a device chain. They also indicate `Body` _kind_ and _length_.
- A `Body` can contain rich data, like user text and media. Data that may be large in size, and that a device may wish to discard or ignore to save on storage or network usage.

== Adding devices

A new device $A_2$ is added by initializing its own device chain, vouching for $A_1$ and having $A_1$ vouch for it. $A_1$ and $A_2$ are siblings.

$2_"A1"$ is an `Entry` from $A_1$ introducing $A_2$ as a sibling. This `Entry` is propagated to Alice's friends, allowing them to recognize $A_2$. Likewise, having $0_"A2"$ vouch for $A_1$'s device chain, allows future friends introduced by $A_2$ to recognize the siblings.

#align(center)[#diagram(
  spacing: (2.2cm, 1.4cm),
  node-stroke: 1pt,
  node-fill: white,
  edge-stroke: 1pt,

  // First device
  node((0,0), [$0_"A1"$], shape: circle, radius: 6mm),
  edge("<|-"),
  node((1,0), [$1_"A1"$], shape: circle, radius: 6mm),
  edge("<|-"),
  node((2,0), [$2_"A1"$], shape: circle, radius: 6mm),
  edge("<|-"),
  node((3,0), [$3_"A1"$], shape: circle, radius: 6mm),
  edge("<|-"),
  node((4.0,0.0), [...etc], stroke: none),

  // Second device
  edge((0,0), (1.5,0.75), "<|-"),
  node((1.5,0.75), [$0_"A2"$], shape: circle, radius: 6mm),
  edge("<|-"),
  node((2.5,0.75), [$1_"A2"$], shape: circle, radius: 6mm),
  edge((2,0), (1.5,0.75), "-|>"),
  edge("<|-"),
  node((4.0,0.75), [...etc], stroke: none),
)]

== Synchronizing <sync>

Let Alice and Bob be users.
Let $A_x$ be a device belonging to Alice, and $B_x$ to Bob.

When Alice wants to synchronize with Bob, the message flow goes like this:

#align(center)[#diagram(
  spacing: (3.2cm, 1.1cm),
  node-stroke: 1pt,
  node-fill: white,
  edge-stroke: 1pt,

  // --- Actors ---
  node((0,0), [$A_1$], shape: rect, width: 2cm, height: 1cm, fill: blue.lighten(75%)),
  node((2,0), [$B_1$], shape: rect, width: 2cm, height: 1cm, fill: green.lighten(75%)),

  // --- Lifelines ---
  edge((0,0), (0,6.5), "-", stroke: 0.8pt + gray),
  edge((2,0), (2,6.5), "-", stroke: 0.8pt + gray),

  // --- Bodys ---
  edge((0,1), (2,1), "-|>", label: [Latest `Entry` from $A_1$ is $Y_"A1"$]),
  edge((0,2), (2,2), "-|>", label: [Latest `Entry` from $A_2$ is $X_"A2"$]),

  edge((0,3), (2,3), "<|-", label: [My latest from Alice is $X_"A1"$ and $Y_"A2"$]),
  edge((0,4), (2,4), "<|-", label: [Send me $X_"A1"$..$Y_"A1"$]),

  edge((0,5), (2,5), "-|>", label: [Send me $X_"A2"$..$Y_"A2"$]),

  edge((0,6), (2,6), "<|-|>", label: [_entries are exchanged_]),
)]

In this scenario, $B_1$ has seen a more recent `Entry` from $A_2$ than $A_1$ has. We allow a device ($A_1$) to synchronize `Entry`s from any sibling ($A_2$) via any friend (Bob). `Entry`s are small and cheap to send, and they are sent from oldest ($X_"A1"$) to newest ($Y_"A1"$), such that the signatures of each `Entry` can be verified immediately on reception.

After both devices are up to speed on `Entry`s, They _may_ ask each other to send `Body`s as well.

Note that devices are not required to comply with synchronization requests. Apps may implement per-friend transfer quotas, to minimize the usefulness of exploiting friends for cheap storage.

== Removing devices

Removing a device is mostly a formality. What it does, is to cap the devices' chain with a tombstone `Entry`, preventing other devices from accepting any new `Entry`s from the removed device

There is, however, no harm in simply "forgetting" a device by locally deleting its private keys, but formally removing it allows other devices to present it as removed, improving UX.

A device can only be removed if it has an active connection to a sibling. Without a sibling, removal would likely be futile since a user might promptly delete the device app, leaving little opportunity for the tombstone message to be propagated to siblings in the normal fashion.

A removal is performed like so:
- The doomed device establishes a direct connection to a sibling device.
- The doomed device appends a tombstone message to its own device chain.
- The doomed device sends it to the sibling device, which is responsible for propagating the tombstone message to friends and other siblings.

A client must reject `Entry`s of other devices for which it has seen a tombstone event, unless those `Entry`s come before the tombstone message in the device chain.

_TODO_ Should we allow a sibling to revoke a device without its cooperation? Would be nice if a device was, for example, stolen.

== Adding Friends

A friend is added by a befriend-`Entry`. The `Body` is encrypted, and includes the device chain root (0th `Entry` and `Body`) of the friend.

When friends are added, two devices from different users exchange their device chain roots (and the root of the original device) over a secure side channel. The 0th `Body` contains a device's public keys, which facilitates secure communication over normal channels.

After the 0th is exchanged, synchronization happens as detailed in @sync.

_TODO_ With befriends/unfriends being present in the device chain, this means that your friends can know roughly how many friends you have. We can avoid this metadata leakage by maintaining a parallel encrypted device chain that we don't share with friends, but sync would be more fragile, since we couldn't bounce the device chain via friends. Presently, we leak this metadata:
  - How many friends we have
  - When any friend was added

== Removing Friends

A friend is removed by an unfriend-`Entry`. Its `Body` is encrypted, and references the `Entry` which added the friend.

#align(center)[#diagram(
  spacing: (2.2cm, 1.4cm),
  node-stroke: 1pt,
  node-fill: white,
  edge-stroke: 1pt + gray,

  // First device
  node((0,0), [], shape: circle, radius: 6mm),
  edge("<|-"),
  node((1,0), [Befriend], shape: circle, radius: 8mm),
  edge("<|-"),
  node((2,0), [], shape: circle, radius: 6mm),
  edge("<|-"),
  node((3,0), [], shape: circle, radius: 6mm),
  edge("<|-"),
  node((4.0,0.0), [...etc], stroke: none),

  // Second device
  edge((0,0), (1.5,0.75), "<|-"),
  node((1.5,0.75), [], shape: circle, radius: 6mm),
  edge("<|-"),
  node((2.5,0.75), [Unfriend], shape: circle, radius: 8mm),
  edge((2,0), (1.5,0.75), "-|>"),
  edge("<|-"),
  node((4.0,0.75), [...etc], stroke: none),

  edge((2.5,0.75), (1.0,0.0), "-|>", stroke: 2pt + red),
)]

When a friend is removed, devices must cease all communication with friend devices.

_TODO_ Inform friend that it has been removed? If we don't do this, a friend might try to connect with you forever in futility.

== Forks & Time Travel

It is possible for malicious or malfunctioning devices to go back and create different, diverging, device chains (a fork). Protecting against this is not a priority, since there is little practical harm in a device attempting to rewrite its history, or trying to present different views to different friends.

Still, for the purposes of local consistency, a device must reject any incoming `Entry`s that do not strictly append to a device chain. This way, forks are prioritized by their time of receipt, with the premise being this: Once a user has seen something (e.g. a post, a comment), it must not change.

Additionally, if a device observes a forked device chain for a friend, the app _may_ choose to automatically generate a unfriend `Entry`, since the friends device is either broken or malicious.

== State resolution

Two siblings may independently change the same user state, creating a conflict. For example, both devices may update the users profile picture. These conflicts are resolved by a vector clock#cite(<vector-clock>), and if the clock is inconclusive, by the lowest `DeviceId`.

When a device receives a new sibling message, it must include the length of its siblings device chain in the next `Entry` it appends to its own device chain. This creates the vector clock.

_TODO_ Should the vector clock use the tail `EntryId` of the sibling chains instead of the length?

= Networking

_TODO_

#bibliography("citations.yaml", title: [References])

