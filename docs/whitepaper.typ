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

A device is described only by a device chain. The device chain is an append-only linked list, where each entry is cryptographically signed using ed25519ph #cite(<rfc8032>). Everything a user does is included in the device chain: Every post, every comment, every friend.

== Types

`Entry` is the root type of the device chain. Postcard is used for serialization.

```rust
type EntryId = Sha512<Entry>;

struct Entry {
  /// Reference to the previous [`Entry`]
  prev: EntryId,
  /// Kind, ID, and length of the [`Body`] of this entry, if any.
  body: Option<(BodyContentKind, BodyId, u64)>,
  /// Tails of sibling device chains. Used as a vector clock.
  // TODO: This makes the `Entry` variable-sized. Having an upper-bound on size would be nice for synchronization guarantees.
  sibling_tails: Map<DeviceId, EntryId>,
}

type BodyId = Sha512<Body>;

struct Body {
  content: BodyContent,
}

enum BodyContent {
  Root { /* ... */ },
  IntroduceSibling { /* ... */ },
  Tombstone { /* ... */ },
  Befriend { /* ... */ },
  Unfriend { /* ... */ },
  Post { /* ... */ },
  Reaction { /* ... */ },
  // etc
}

/// The Device ID is the Entry ID of the device chain root node.
type DeviceId = EntryId;
```

The ID types are SHA512 hashes of the identified value.
Each `Entry` references its own `Body` by ID, as well as the previous `Entry`. This creates a linked list of entries -- the device chain, which facilitates a common view from any given device. More details in @sync.

#figure(
  caption: [A Device Chain],
  diagram(
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
    edge((0,0), (0,1), "=|>"),
    node((0,1), [`Body`], shape: circle, radius: 6mm, fill: blue.lighten(70%)),
    edge((1,0), (1,1), "=|>"),
    node((1,1), [`Body`], shape: circle, radius: 6mm, fill: blue.lighten(70%)),
    edge((2,0), (2,1), "=|>"),
    node((2,1), [`Body`], shape: circle, radius: 6mm, fill: blue.lighten(70%)),
    edge((3,0), (3,1), "=|>"),
    node((3,1), [`Body`], shape: circle, radius: 6mm, fill: blue.lighten(70%)),
  ),
  )

  `Body`s and `Entrys` serve different purposes:
- An `Entry` is small and signed. It's used to quickly synchronize the shape of a device chain. They also indicate `Body` _kind_ and _length_.
- A `Body` can contain rich data, like user text and media. Data that may be large in size, which a device may wish to discard or ignore to save on storage or network usage.

== Adding Siblings

A user who has a device $A_1$ may introduce a second device $A_2$ by having it initialize its own device chain, introducing $A_1$ and having $A_1$ introduce it. $A_1$ and $A_2$ are siblings. Sibling-introductions are a special `Body` which includes the siblings `DeviceId` (_TODO_ and public keys?).

In @add-sibling, $2_"A1"$ is an `Entry` from $A_1$ introducing $A_2$ as a sibling. This `Entry` is propagated to Alice's friends, allowing them to recognize $A_2$. Likewise, having $0_"A2"$ vouch for $A_1$'s device chain, allows future friends introduced by $A_2$ to recognize the siblings.

_TODO_ Should a hypothetical sibling $A_3$, also introduced by $A_1$, strive to vouch for $A_2$ in its device tree to make sync easier? Perhaps a device must vouch for its sibling before including it in its vector clock?

#figure(
  caption: [Introducing a Sibling],
  diagram(
    spacing: (1.8cm, 1.0cm),
    node-stroke: 1pt,
    node-fill: white,
    edge-stroke: 1pt,

    // First device
    node((1,0), [$0_"A1"$], shape: circle, radius: 6mm),
    edge("<|-"),
    node((2,0), [$1_"A1"$], shape: circle, radius: 6mm),
    edge("<|-"),
    node((3,0), [$2_"A1"$], shape: circle, radius: 6mm),
    edge("<|-"),
    node((4,0), [$3_"A1"$], shape: circle, radius: 6mm),
    edge("<|-"),
    node((5.0,0.0), [...etc], stroke: none),

    // Separator
    edge((0,-0.5), (5,-0.5), "--", stroke: 0.8pt + gray, label: [*Alice*]),
    edge((0,0.5), (5,0.5), "--", stroke: 0.8pt + gray, label: [_Device $A_1$_], label-pos: 0.055),
    edge((0,1.5), (5,1.5), "--", stroke: 0.8pt + gray, label: [_Device $A_2$_], label-pos: 0.055),

    // Second device
    edge((1,0), (2.5,1), "<|-"),
    node((2.5,1), [$0_"A2"$], shape: circle, radius: 6mm),
    edge("<|-"),
    node((3.5,1), [$1_"A2"$], shape: circle, radius: 6mm),
    edge((3,0), (2.5,1), "-|>"),
    edge("<|-"),
    node((5.0,1), [...etc], stroke: none),
  ),
) <add-sibling>

== Synchronizing <sync>

Let Alice and Bob be users. When Alice wants to synchronize with Bob, she must first establish a line of communication, this is described in @networking. @sync-friend gives an example of a subsequent conversation between Alice's device $A_1$, and Bob's device $B_1$. In this example, Alice has a second device $A_2$.

#figure(
  caption: [Synchronizing `Entrys` with a friend],
  diagram(
    spacing: (2.5cm, 1.1cm),
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
    edge((0,1), (2,1), "-|>", label: [What is your view of Alice?]),
    edge((0,2), (2,2), "<|-", label: [My view of Alice is $X_"A1"$ and $Y_"A2"$]),
    edge((0,3), (2,3), "-|>", label: [Send me $X_"A2"$..$Y_"A2"$]),
    edge((0,4), (2,4), "-|>", label: [My view of Alice is $X_"A2"$ and $Y_"A1"$]),
    edge((0,5), (2,5), "<|-", label: [Send me $X_"A1"$..$Y_"A1"$]),
    edge((0,6), (2,6), "<|-|>", label: [_entries are exchanged_]),
  ),
) <sync-friend>

In @sync-friend, $B_1$ has seen a more recent `Entry` from $A_2$ than $A_1$ has. We allow a device ($A_1$) to synchronize `Entry`s from any sibling ($A_2$) via any friend (Bob). `Entry`s are small and cheap to send, and they are sent from oldest ($X_"A1"$) to newest ($Y_"A1"$), such that the signatures of each `Entry` can be verified immediately on reception.

After both devices are up to speed on `Entry`s, They _may_ ask each other to send `Body`s as well.

Note that devices are not required to comply with synchronization requests. Apps may implement per-friend transfer quotas, to minimize the usefulness of exploiting friends for cheap storage.

If a device want's to append `Entry`s to its own device chain, it should generally defer from doing so until it has synchronized with at least one other device (friend or sibling). This can look like @sync-friend, where Alice's device starts by asking _Bob's_ device about _its_ view of Alice's devices. This, way, the device has a more up-to-date view of its siblings when the `Entry` is added, which can help reduce state conflicts as described in @state-resolution.

== Removing Siblings

Removing a device is mostly a formality. There is no harm in simply "forgetting" a device by locally deleting its private keys, but formally removing it allows other devices to present it as removed, improving UX.

A device can be removed in two ways, but both ways require a sibling.

The first way, is for the device to append a tombstone `Entry` to its device chain. Other devices must reject `Entry`s that append to a tombstone `Entry`, so this effectively finalizes the device chain.

A device can remove itself iff it has an active connection to a sibling. Without a sibling, removal would likely be futile since a user might promptly delete the device app, leaving little opportunity for the tombstone `Entry` to be propagated to siblings (or friends) in the normal fashion.

A self-removal is performed like so:
- The doomed device establishes a direct connection to a sibling device.
- The doomed device appends a tombstone message to its own device chain.
- The doomed device sends it to the sibling device, which is responsible for propagating the tombstone message to friends and other siblings.

The other way to remove a device, is by its sibling issuing a sibling-revocation-`Entry`. This can be useful when a device is lost. The sibling-revocation-`Entry` references the siblings `DeviceId`, and its latest known `Entry`.

== Adding Friends
A friend is added by a befriend-`Entry`. The `Body` is encrypted, and includes the device chain root (0th `Entry` and `Body`) of the friend.

When friends are added, two devices from different users exchange their device chain roots (and the root of the original device) over a secure side channel. The 0th `Body` contains a device's public keys, which facilitates secure communication over normal channels.

After the 0th is exchanged, synchronization happens as detailed in @sync.

_TODO_ With befriends/unfriends being present in the device chain, this means that your friends can know roughly how many friends you have. We can avoid this metadata leakage by maintaining a parallel encrypted device chain that we don't share with friends, but sync would be more fragile, since we couldn't bounce the device chain via friends. Presently, we leak this metadata:
  - How many friends we have
  - When any friend was added

== Removing Friends

A friend is removed by an unfriend-`Entry`. Its `Body` is encrypted, and references the `Entry` which added the friend.

#figure(
  caption: [`Befriend` and `Unfriend`],
  diagram(
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
  ),
) <befriend-unfriend>

_TODO_: Improve @befriend-unfriend

When a friend is removed, devices must cease all communication with friend devices.

_TODO_ Inform friend that it has been removed? If we don't do this, a friend might try to connect with you forever in futility.

== Forks and Time Travel

It is possible for malicious or malfunctioning devices to go back and create different, diverging, device chains (a fork). Protecting against this is not a priority, since there is little practical harm in a device attempting to rewrite its history, or trying to present different views to different friends.

Still, for the purposes of local consistency, a device must reject any incoming `Entry`s that do not strictly append to a device chain. This way, forks are prioritized by their time of receipt, with the premise being this: Once a user has seen something (e.g. a post, a comment), it must not change.

Additionally, if a device observes a forked device chain for a friend, the app _may_ choose to automatically generate a unfriend `Entry`, since the friends device is either broken or malicious.

== State Resolution <state-resolution>

Two siblings may independently change the same user state, creating a conflict. For example, both devices may update the users profile picture. These conflicts are resolved by a vector clock#cite(<vector-clock>), and if the clock is inconclusive, by the lowest `DeviceId`.

When a device receives a new sibling `Entry` from a sibling, it must, if possible, include the tail `EntryId` of the sibling device chain in the next `Entry` it appends to its own device chain. By referencing earlier `Entry`s from siblings in this fashion, we establish a partial ordering of `Entry`s. This is possible because the set of all device chains of a user make up a directed acyclic graph.

Furthermore, the vector clock is transitive: Device chain $A_1$ must not reference the latest entry of its sibling $A_2$, without also referencing the latest entry of _every additional sibling_ referenced by $A_2$.

= Networking <networking>

_TODO_

#bibliography("citations.yaml", title: [References])

