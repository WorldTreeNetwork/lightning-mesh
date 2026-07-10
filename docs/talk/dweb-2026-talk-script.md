# Locked Open: How To Build Systems With No Authority To Capture

How To Decentralize Anything In 3 Easy Steps

**Status:** Stage script, in progress — written beat by beat against
[the outline](dweb-2026-talk-outline.md) | **Register:** spoken word, first
person | **Convention:** *[bracketed italics]* are staging/delivery cues, not
spoken. | **Style rule:** simplicity is the aesthetic of confidence. The material earns attention by being true and vivid,
not by asking for it. Enthusiasm and vivid imagery: yes.
And: **technologist, not showman.** No "this one's my favorite," no "watch
this" framing — rhetorical devices guide awareness (structure, contrast, a
well-placed image); they don't sell the next paragraph. Clarity and precision
are the register; color comes from the material being genuinely strange and
good.

---

## Beat 1 — Cold open: the demo (~5 min)

*[Before you're introduced, if logistics allow: the venue mesh routers are
already up. The slide behind you shows only two lines, big:]*

> **Join wifi: `Lightning Mesh`**
> **Open: `http://hello.mesh`** *(yes, type the `http://`)*

*[Walk out. Don't introduce yourself yet.]*

There's a wifi network in this room called
`lightning`. Join it. Then open your browser and go to `http://hello.mesh`.
You have to type the `http` part — your browser doesn't believe this website
exists. 

*[Beat. Let people fumble with phones. Switch the screen to your own view of
hello.mesh — the live presence page.]*

While you do that, watch the screen. Every one of those entries appearing —
that's one of you. That's you joining a network. Not my network — a network.
One that you're now a first-class citizen of. And notice — it's
bigger than it was a minute ago *because* you joined. You're not entering a
thing that already exists. You're creating it. Right now, together.

*[As identities populate:]*

What you're looking at is the network seeing itself. Each row is an identity —
a cryptographic key that your browser just created, on your phone, by itself.
Nobody issued it to you. Nobody approved it. There's no account. And here's a
question worth asking about any web page: *what server is this running on?*
The answer here is: whichever router is nearest to you. Every router in this mesh runs a
tiny web server — the front desk — and serves this same page from itself. The
little liveness dot next to each name — who's here, right now — isn't coming
from *a* server, because there is no *the* server. Every router in this room
holds this same list, merged via a CRDT, and they agree on it without any of them being in
charge.

Now for the second half of the demo.

*[Hold up, or point to, the demo box — an ordinary small computer (SBC or the
talk laptop) wired to one of the mesh routers.]*

This is a completely ordinary computer. Earlier today I plugged it into one of
the mesh routers here — the same way you'd plug anything into the router
behind your couch. I didn't configure it. No port forwarding, no dynamic DNS,
no VPS in the middle, no Tailscale account, no certificate. I plugged it in,
and it announced a name. You can see it from where you're sitting — it's the
app linked right there on the front desk page.

*[Tap the link on hello.mesh — the chat / walkie-talkie. If shipped: send a
message to the room, or take a p2p voice call from a phone's browser. One
interaction, fast, no dwelling.]*

That app is running on *this box*, and its name resolves from every node on
this mesh. And if my home mesh were linked in, it would
resolve from my house too. From anywhere on Earth, over an encrypted overlay,
by the same name.

That's nice. Plenty of products will sell you that. Here's what no product
sells you.

*[The turn. Slow down.]*

Cut the internet.

*[If staged live: actually pull the venue uplink, or kill the upstream on the
gateway router. Let the presence page keep ticking.]*

Nothing happened. Look at the page. Everyone's still there. The name still
resolves. If the box is serving a chat, the chat still works. Because none of
this depends on the internet being there. As long as these radios can
reach each other — hop by hop, across the room or across a valley — you can
plug a computer into any router in the mesh and every other node can reach it,
by name, exactly the same way. No egress required. No permission required.

*[Beat.]*

There are people today — and I'll come back to this near the end —
for whom "the internet got cut" is not a hypothetical. It is a weapon that is
being used against them, right now. 

So here's what this talk is about. Everything you just watched is
*structurally impossible* on the regular internet you use every day. Not difficult —
impossible, by design. Because at every single layer of the modern stack,
there's an **authority** baked in: something that grants you an address, grants
you a name, grants you trust, grants you an identity. And whatever is granted
can be taken away - rented to you, surveilled, or switched off.

What I want to show you a
portable method for finding those hidden authorities in any system, and
removing them. We applied it to networking. You can apply it to whatever
you're building. And I want to show you the strange, wonderful thing that
falls out when you do: a network with no center and no configuration necessary. 
A network that structurally can't be owned.

My name is Duke. Let's get into it.

*[Slide: talk title. Only now.]*

## Beat 3 — Learning to see the authority (~6 min)

*[Title slide is up from the end of beat 1. Advance to a plain slide:
"Whose permission am I asking for right now?"]*

SPOF

I want to start with something that's designed to be invisible.

What just happened, at a protocol level, when your
phone joined the wifi just now — this one, or any network? Your phone's
first act, before it can say anything to anyone, is to ask permission to
exist. It broadcasts, essentially: *is there an authority here? May I have an
address?* That's DHCP. And somewhere, a server — one particular box, with one
particular config file — decides what you're called and whether you're allowed
on. Your device can't talk on the network until someone in charge
says yes.

*[Slide: Centralizing Control. Reveal one rung at a time.]*

Services: you provide something for your community. Chat. Information: a wiki. 
Call each other. The Schedule for this conference. Tools to Coordinate with each other.

You want a **name** — something people can find you by? Names are granted by a
registrar. You rent it. Stop paying, and someone else will buy it — along with
everyone's links to you.

You want to be **trusted** — stand up an internet service, maybe one to help
your community, you want to get that little padlock in your browser - encrypted
- so nobody can see you.  Trust is granted by a
certificate authority. A company on an official list, baked into your
browser by a vendor you didn't choose. The padlock doesn't really mean "this is
safe." More like, "some authority vouched."

You want to **be someone** — to log in? Being someone is granted by an issuer.
Sign in with Google, sign in with Apple. Your ability to be yourself on the
internet is a service, provided to you, revocably, by a company.

And where do you get your **phone number**? You lease it from a
carrier. They check your gov't ID, issue you a number. And that's how you're reached.
Every app demands it as proof you're real. And it is also, not by
accident, the single best surveillance and tracking handle that has ever
existed. The thing you're *called by* is a tracking device you pay for
monthly.


Address, name, trust, identity. Four layers, and at every one, the same shape:
something you're made to think is *yours* is actually **granted** to you. 
But it's not that the people running these authorities are bad.
The problem is structural. **Every grant point is a locus of control.** 
A locus of control is a *lever* - and a lever is always used, eventually,
so long as the authority has something to gain by it - an incentive.

Control gets used as *rent*: you can't reach your own computer behind your own
router without paying somebody — a cloud provider, a tunnel service — for the
privilege of reachability. 

Control gets used as *surveillance*: the phone number, the account, the
certificate log — every grant is data that lives in a registry, a map
of who you are and who you talk to. We all know that collected data gets sold, and used.

And control gets used as a *censor*: whatever is granted to you can be revoked. An
account, a name, a route, a whole region's connectivity. 

So are these things really yours?

*[Beat. Shift weight — history now.]*

The internet's original design — the
actual founding requirement of the ARPANET — was *survive the loss of your
center*. A network that keeps working when any part of it is destroyed. That
was the whole point. And then we spent forty years  recentralizing onto a handful of clouds, a handful of CAs,
a handful of corporate platforms — and now, a few well-placed failures, or decisions, can
switch off enormous swaths of it. We took the one network designed to survive
decapitation and gave it a head.

*[Beat. Then the pivot — energy up, this is the turn to possibility.]*

Now — none of this is new. 
A whole movement already named the cure: own your identity, own your data,
permissionless participation, services no one can enclose. 
You may recognize this: the
promise of web3. Sounds good, right? And then it strapped a
speculative asset on and set the whole thing on fire with grift and greed.

So here's the root claim of this talk: **We can Keep the original promise 
of web3 without a token or a blockchain** Everything web3 said we'd get — 
a network we own, that no one can
capture — is deliverable with only three ingredients, none of which 
requires a speculative asset.

Self-generated **keys**: identity you mint yourself. Nobody issues it, so
nobody can revoke it.

**Peering** — in the technical sense, this is a word that already means
exactly the right thing. In internet engineering, *peering* is the
relationship between networks that connect as **equals**: settlement-free, no
money changes hands, because neither one is above the other. Its opposite is
*transit* — the relationship where you're a customer. The internet's
backbone already runs on peering. The giants peer with each
other — *transit is what they sell to the rest of us.* So ingredient two is
simply: give every node the relationship the backbone reserves for itself.
Your home router peers. A Raspberry Pi peers. A phone peers. Nobody in this
network is anybody's customer.
*[ pic: peer vs bus ]*

And the third ingredient: **convergence**. You ever seen `conflicted copy
(2)` show up in your Dropbox? That's sync when it doesn't know *whose version wins?* - and you have to figure it out. 
Convergence is sync that always has the answer.
Every node computes the same winner, independently, by math. And this is allows a symmetric protocol. 
No leader, no authority. Everyone makes changes independently and then arrives at the same state together. Split the
network in half, let both halves live separate lives for a week, rejoin them —
they converge. No referee, no merge conflict.

**Keys, peering, and convergence.** No blockchain, no
speculative asset in the path. A checklist — three questions
you can point at any system that stands between people, that intermediates: *Who mints your
identity? Are you a peer, or a customer? And who decides whose version wins?*
Every centralized service provides one kind of answer to those three questions. 
What I'm about to show you provides a very different kind of answer.

And now, we get to look at how those three ingredients
dissolve every point of centralizing control we went over — the address, the name, the trust, the
identity. But how did I figure this out?  I found each one by being bitten by it.

---

## Beat 4 — The road here: I built a gate by accident (~5 min)

*[Slide: one word — "IdentiKey". Register shift: slower, personal. This is
the confession the last line promised.]*

Some years ago I set out to build what I thought was the smallest ingredient
of the three. Just the keys.

The project is called IdentiKey: identity rooted in cryptographic keys
instead of accounts. The idea is old: **you are your key.** Not
"you are row 40,000 in someone's user database" — you are the holder of
something you generated yourself, and the network's only job is to verify
signatures. The right to *be someone* online without asking
permission to exist.

And to get something working on a deadline, I did the pragmatic thing.

I stood up an OAuth server.

And it worked! Login worked, the apps functioned, no login with google. 
But I had actually built an issuer. My own authority. I *became* the SPOF.
A place that says yes or no. A thing every login phones home to. what happens if I switch off the server?
**I had set out to abolish the gatekeeper, and the first working thing I built was
another gate.**

Was I just stupid? Maybe a bit, but I don't think that's all of it --
It's that every part on the shelf
is authority-shaped. Every major spec, every library, every protocol you reach
for when building these sorts of things was designed with authoritative model as a 
starting assumption. It's a kind of hack, just talk to the boss, it solves the problem for now, 
you never come back to it.  You want to "just verify who someone is" and your hand closes around a
certificate authority, a domain name, a relying-party server, some platform
vendor. You can't assemble those parts into anything but a gate. The
centralization isn't in the products. It's in the *parts*.

*[Beat.]*

Meanwhile — because apparently one impossible project isn't enough — I was
also building a decentralized compute system called Mjolnir: programs as
processes spread across many machines, cooperating with no central scheduler.
Process calculus! Distributed Computation!
Islands of servers in different places, and I needed them to
reach each other, I didn't want to configure it manually every time a new server came up
so I built some network plumbing to stitch the islands
together. It was supposed to be a weekend of glue.

*[Small beat. The turn.]*

Here's what all those failures taught me. Every time I got the identity design
close — really close — I'd find I had leaned on some centralizing assumption. A CA being reachable. 
A DNS name. A cloud database holding the canonical copy. And each
time, the censorship-resistance I was promising vanished — not because the
crypto was weak, but because **the chokepoint moves to the SPOF.** You
don't have to break a key if you can revoke a certificate. You don't have to
forge a cryptographic signature if you can seize a server.

A key is *what you hold*. But holding means nothing if there's nowhere to
relate, nowhere to arrive. A key with no commons is a ticket to an empty theater
that doesn't exist (as we learned in crypto).

So the conclusion, when I finally accepted it: **I had to build the ground
first.** A network with no authority in it anywhere — not hidden, not
deferred, none. And to build *that*, I had to build it somewhere no
authority could be leaned on even if you wanted to: wifi radios in the middle 
of the forest, nodes that get unplugged, networks that split in half then reappear, 
and have to keep working like nothing happened.
No coordinator, no CA, and certainly no cloud. The mesh isn't a pivot away from identity. It's
the only solid foundation for a sovereign identity, poured in the most adversarial soil I could find. 
Because anything that grows there doesn't need anyone's permission
to keep existing.

The weekend of glue became this project. And what came
out of it was surprising, to say the least.

---

## Beat 5 — The method (~5 min)

How To Decentralize Anything In 3 Easy Steps

*[Slide: three words — keys · peering · convergence.]*

What came out is a method with three steps. You've met them as concepts;
here's what each one looks like in reality — and what it gets us.

**First step: every node runs identical software.** There is no
controller build, no server edition, no admin mode. This is peering made
real, and it's stricter than it sounds. It's not that we have a good way to 
elect a leader — the system is designed with no role a leader could even occupy. 
The moment your design includes one special player, you've built a throne, and history says
somebody sits in it. Symmetry makes sure we're all on equal footing.

**Second state: shared state converges by math.** Every node keeps a
small database — who's on the mesh, which addresses are claimed, who has this name. 
Nodes trade entries with their neighbors, a gossip network. No node ever 
has the whole truth first; but every node ends up with the
same truth eventually.

But what happens when there's a conflict? Two nodes, out of contact, claim the same
name. When the networks touch again, who wins? Every entry carries a stamp:
the writer's wall-clock time, a counter, and the writer's public key. You
compare stamps the way you compare words in a dictionary: earlier clock wins;
if the clocks tie, the counter breaks it; if those tie, the key breaks it —
and keys are unique, so there is always exactly one winner. Any node can run
that comparison — this week, next month, on another continent — and gets the
same answer. "Who decides?" turns out to have the most boring possible
answer: *everyone, identically.* The literature calls this a CRDT with a
hybrid logical clock. The whole trick fits on one slide, and it replaces the
referee.

**Third step: nothing is allocated from an authority.** Anything that must be unique — an
address, a name — is either derived from a key or claimed-and-converged. An
allocator is an authority with a spreadsheet, so the method forbids it: if
something needs handing out, redesign it until it doesn't.

*[Beat. Slide: an empty config file.]*

And here's what the three commitments buy, together. Think about what
configuration *is*. Every line of network config you've ever written points
at an authority: which box is the DHCP server, what range it hands out,
which controller to enroll with, which CA to trust. **Configuration is the
paperwork of authority.** Remove the authorities and the paperwork doesn't
get easier — it disappears. Plug a node in: it mints its own identity, claims its
address space, gossips out what it knows, and magically converges with everyone else. 
Unplug it: the mesh adapts. Nothing was set up, because there is nothing to set up. And
notice what that means for the network as a whole: it grows by the plugging
in. Joining doesn't just *use* the commons — joining is what the commons is
made of.

*[Beat. Slide: 1974 — a photo of the SRI packet radio van, if you can get
one.]*

I want to be clear that none of this came from nowhere. In 1974, Vint Cerf
and Bob Kahn had a pile of networks that couldn't talk to each other — leased lines,
satellite links, literal radio vans driving around the Bay Area — and their
answer was: don't unify the links, unify a new layer above them. They called
it the catenet. It's the only network design that has ever scaled five
orders of magnitude, and it's the shape we build.

And for the last twenty years, community networks kept that flame when
almost nobody else did — Freifunk, Guifi.net, NYC Mesh, the CeroWrt folks.
They ran collaboratively-owned networking at real scale, hit the walls
first, and converged on this same
shape: small link islands, stitched by routing, no SPOF, unified.
And meshes built on these
principles can interconnect; making our networks complementary is work
we're doing right now.

So: three commitments — identical software, convergence by math, nothing
allocated. Now let's climb back down the stack from the beginning of the
talk and dissolve it, layer by layer. Starting with the address.

---

## Beat 6 — First rung: the address (~6 min)

*[Slide: the ladder returns, rung 1 highlighted — "address: granted".]*

DHCP. The protocol that answers your device's very first question: *may I
have an address?*

Here's how it works everywhere today: one box on the network holds a config
file and a lease table. Your device broadcasts into the dark, that box
answers, and whatever it says, goes. That box is the network's memory —
every device's existence is a row in its table, it's the authority + chokepoint.
And if there's ever more than one router that thinks it's the authority, things
start breaking.

Anyone who has ever set up a network knows the feeling this produces:
somewhere in the mysterious stack, something isn't configured right — and
nothing works at all. No error, no sign. Just
silence, and your weekend gone. Most of that mystery traces back to the allocation
points: some authority, somewhere, that didn't get told the right thing.
DHCP alone supplies a whole catalog — the home router plugged in backwards,
handing out addresses to half the office; two networks that both
claimed `192.168.1.x`, so joining them together means one side gets to renumber its
entire world. Manually configured.

That last one points at a general truth: **authorities don't merge.** Two
adjudicators with overlapping jurisdiction can't compose into one network —
one of them has to lose, and a human has to step in and negotiate the
treaty. Every boundary between networks is painful *because* there's an
authority on each side of it. That's not a DHCP bug. It's the cost of authority.

*[Beat. Slide: rung 1 dissolving — "address: claimed & converged".]*

So here's the move. Delete the server. Don't replace it.

In the mesh, each router claims its own block of addresses — a /24 of its
own — and writes that claim into the shared state, the same gossiped
database from a few minutes ago. The claim spreads node to node. Every
router ends up knowing every block: who claims it, stamped how. And if two
routers ever claim the same block — nobody panics and nobody negotiates. It's just a
conflict in the database, and we already know exactly what to do:
the stamps get compared, everyone computes the same winner, and the loser
derives a new block and moves on. Automatically. In seconds.

And look at what falls out. Plug in a router: it addresses itself, claims
its block, starts routing. No DHCP range to plan, no spreadsheet of addresses, no
authority to designate — empty config file. Unplug it: the mesh adapts. 
And plug two *whole meshes* into each
other — two networks with separate histories — and the it self-heals.

Claims merge, authorities don't. 

One more thing falls out: every router owning its own block means
every router owns its own segment. A misbehaving device's broadcast chatter,
an ARP spoofer, somebody's cursed IoT gadget — the blast radius ends at the
segment boundary, structurally. Containment isn't a firewall rule someone
maintains. It's the shape of the network.

---

## Beat 7 — Second rung: the name, and being reachable at all (~9 min)

*[Slide: the ladder, rung 2 highlighted — "name: granted".]*

The name rung is really two problems wearing one coat. A name has to *mean*
something — that's the directory problem. And the thing it points to has to
be *reachable* — and on today's internet, that second part is quietly broken
for almost everyone.

Think about what it takes, today, for you to run something as small as a
photo album for your family, or a scheduling app for your anarchist collective. 
Your computer sits behind your router - a NAT, its own private house network.
It has no address anyone can dial from outside. So you either become a network wizard —
port forwarding, dynamic DNS, TLS certificates — or you pay the problem to
go away: a VPS, a tunnel service, somebody's cloud. An entire industry
exists to rent you back the ability to be reached. Not even hosting anything —
just to be *reachable*. That's transit-thinking all the way down: you, poor sap
asking the network for permission to answer a phone call.

Here's the other way, and it's a truly great piece of software. It's called Iroh, it's open
source and usable in your projects today.

**Your public key is your address.** Not "your key maps to an address" —
the key *is* the address. You dial a key. The network figures out
how to get there: same room, it goes over the local radio; across the
mesh, it hops the radios; across the world, it traverses the NATs and rides
the open internet — an encrypted QUIC connection, end to end, between two
identities. The routers in between, mine, yours, a café's — they carry
ciphertext they cannot read. You get a trusted connection over hardware
nobody has to trust.

Notice what that means for the question "where are you?" It stops having a
network answer. Behind a NAT, on a mesh island, on fiber in a datacenter —
those become the connection's problem, solved underneath you. Dial the key;
the dial is identical in a disaster zone with no internet and on a gigabit
line with full internet. One system for both worlds — the abundant one and
the deprived one — which is the property everything later in this talk
stands on.

*[Slide: rung 2 dissolving — "name: claimed & converged".]*

So reachability comes from keys. But nobody wants to dial a key, any more
than they want to memorize a quadruple phone number. Names.

names are just entries in the same gossiped database as the address blocks. When the demo box joined this
morning, it wrote a claim: this name, this key, this stamp. The claim
converged router to router; now every node resolves it locally, from its own
copy. No name server to run, no registrar to pay, nothing to seize. Split
the mesh in half and both halves keep resolving every name they knew;
rejoin, and the claims converge like everything else.

Today a name belongs to whoever claims it first
— first-writer-wins, arbitrated by the stamps. 
because every claim is attested by a key, names can accumulate *reputation* —
vouches, webs of trust — so "first to claim" matures into "first
*legitimate* claimant." And notice that's the same machinery for a router,
a service, and a person. One identity method, all the way up. That's the
next rung, and we'll get there in a minute.

*[Point back at the presence page — the liveness dots.]*

One more piece of this rung: the little dot that says who's here *right
now*. It looks trivial, and it hides a hard problem.

Our shared database stores facts — this key claims this name, this stamp.
Facts merge; that's the whole trick. But "Alice is here right now" is not
that kind of fact. It's true, and then it *stops* being true, and
no event marks the moment. There's nothing to merge, because absence
doesn't write. You cannot gossip your way to "she's gone."

So presence doesn't live in a database at all. It rides a separate,
forgetful channel: tiny beacons, sent often, never stored,
never relayed, never merged. And each node judges staleness by its *own*
clock — I trust your clock to order your writes, but I will never use your
clock to decide whether you're still breathing. Durable truth in one plane,
perishable truth in the other, and neither pretends to be the other. Most
of the painful bugs in distributed systems come from mixing those two up.

*[Beat. Status, plainly.]*

Where this stands in the real world: the shared database and the address
book have been running on a real router fleet for months. One field report:
a router was powered off through an entire fleet software
update. It came back days later, gossiped with its neighbors, and converged
in seconds. Nobody noticed. Nobody had to. The names and services layer is
younger.

And that closes the loop on the opening demo. The box I plugged in this
morning: its name was a claim that gossiped, its reachability came with its
key, and the internet's presence or absence never entered into it.
**Reachability stopped being a product you subscribe to and became a
property of joining.** That's rung two.

---

## Beat 8 — Top of the ladder: trust, and being someone (~8 min)

*[Slide: the ladder, rungs 3 and 4 highlighted — "trust: granted" and
"identity: granted".]*

The top two rungs — trust, and identity — are really one rung viewed from
two sides. Both come down to the same question: *who vouches for you?*

Today the answers are: a certificate authority vouches for your server, and
an issuer vouches for you. Sign in with Google. Sign in with Apple. Verify
with a text message to — there it is again — your phone number. The
identifier you're reachable at is leased from a carrier, demanded by every
service as proof you're a person, and joined against every database you
touch. We built a world where *being someone* is a service: provided to
you, rate-limited, terms-of-service'd, and revocable. And the identifier at
the center of it doubles as the best tracking handle ever devised.

This rung is where the whole project started for me, so here is IdentiKey's
answer, and it fits in one sentence: **the protocol verifies only
signatures.**

Your identity is a keypair you generate yourself. Each of your devices
holds its own key, and your identity key signs a short statement — *this
device key acts for me* — that anyone can check. Checking it is pure math:
no issuer to call, no registry to consult, no home to phone. A router in
this room can verify that your phone speaks for you while the mesh is
split from the internet, from me, from everything — because verification
needs nothing but the signature and the key in front of it. That one rule,
held everywhere with no exceptions, is what it actually takes for "nobody
can switch your identity off" to be true rather than aspirational.

We did look hard at passkeys — they're real cryptography and a real
improvement. But a passkey is bound to a DNS domain and to a platform
vendor's account system, which is to say: to two authorities. Adopting
them would have welded the top of the ladder back onto the bottom. We
declined.

*[Beat.]*

Two consequences of "only signatures," and they're the ones I care about
most.

First: **anonymity is a first-class citizen.** A keypair minted for one
conversation and thrown away afterward is a valid identity — not
a degraded mode, not a suspicious edge case. The protocol doesn't know the
difference between a throwaway key and a lifelong one, and that's by
design. You all did this, forty minutes ago: every identity on that
presence page was minted by a browser, no email, no phone number, no
CAPTCHA. From there it's a spectrum you navigate by choice — keep the key in
your browser, move it to an app, put it in hardware, or hand custody to
someone you trust who runs an ordinary service on the mesh. Every rung of
custody looks identical to every service: a key, and valid signatures.

Second: **there is no registry.** This one is vital. Every identity system accumulates a database, and
every such database is a liability with a timer on it — because a registry
read backwards is a map of people: who exists, who talks to whom, how to find them. 
Companies get sold. Registries change hands. Here, there is nothing to seize, because
identity was never *recorded* anywhere — it's verified pairwise, offline,
at the moment of use, and forgotten. An identity system with no panopticon
in it, because the panopticon was never built. Near the end of the talk
I'll tell you about the people who taught me how much this matters.

*[Beat. Slight smile — the payoff of the oldest setup in the talk.]*

Now. At the very start, I made you type `http://`, and your browser
refused to believe the site was real. Here's what that was.

Browsers gate their own cryptography — `crypto.subtle`, the good API, the
one with non-extractable keys — behind a *secure context*,
which in practice means HTTPS with a certificate from an authority on the
browser's list. Refuse the certificate authority, and the browser turns
its crypto off. So the most
secure API in the platform becomes unavailable at precisely
the moment you decline the landlord. On a mesh with no CA, the
browser — your one universal client — shows up with its best tools
disabled.

We climb out with a ladder of our own. In the bare browser, we ship a
small, audited pure-JavaScript Ed25519 - we
call it soft custody, and it's plenty for
saying hello. Future work, a browser extension *is* a secure context
— full hardware-backed WebCrypto, no CA involved. And between mesh-native
software, the problem doesn't exist at all: an iroh connection uses the
node's key *as* its TLS identity, so every hop you saw tonight was already
encrypted, mutually authenticated, and certificate-free. 

The gate is real, the gate is annoying, and we can hop the fence.

*[Slide: the full ladder, every rung dissolved — "granted" struck through,
"self-created / claimed / converged" beneath.]*

Step back and look at what happened to the ladder. Your address: derived
from your key. Your name: claimed via your key. Trust: a signature check.
Being someone: holding a key. Every "granted" became "created" or
"claimed" — and the network, all of it, became a projection of a set of
keys. The physical substrate — which radio, which building, which
continent — is routing detail underneath the thing that actually matters:
*who*, cryptographically, is talking to *whom*.


## Beat 9 — What falls out (~3 min)

*[Slide: blank, then one line at a time as each property is named.]*

Before the last part of this talk, I want to collect what the method
produced — because the striking thing is that none of these were features
we set out to build. Each resulted from engineering out an authority to make the system
*actually* sovereign.

**Nothing to configure.** Not simplified setup — no setup. Configuration
was the paperwork of authority, and the authorities are gone.

**Networks compose.** Two meshes with separate histories meet, and the
seam heals: claims converge, routes stitch. Growth has no admission
process, so the commons grows by whoever shows up.

**Partition is not failure.** Split this network in half, and both halves
keep working — resolving names, routing packets, serving the room — on
whatever they knew last. When the halves find each other again, they
gossip and converge, and no one resyncs against a primary, because there
is no primary. Most systems treat partition as the disaster case and hope
it's rare. Out here it isn't rare — a radio mesh partitions *constantly* —
so it's the normal case, engineered to be boring. A network that only
works while it's whole hasn't been tested yet.

**A P2P Network Is A Commons** Joining it, you contribute to it. 

And the sum of the four: **nobody is in charge — and nobody is in a
position to *become* in charge.** Not because of governance, or a
foundation, or good intentions. There is no role to seize, no registry to
subpoena, no server to acquire. Sovereignty here isn't a policy. It's the
topology.

*[Slide: the principles, assembled — permissionless, self-sovereign,
disintermediated, symmetric, censorship-resistant, resilient... and a
seventh, dimmed, unnamed.]*

That's the vocabulary of this movement, earned one mechanism at a time —
and there's one word left on the list. I can't get to it through
engineering. I have to get to it through a story, and it's the one I
promised you at the beginning.

---

## Beat 10 — Why it matters (~5 min)

*[No slide, or a plain dark slide. Stand still. Plain voice — no
performance in this beat at all.]*

Some of you know that people from Myanmar have been part of this community
for years. They come to DWeb working on a problem most of us have never
had to pose: how do you run the services a society needs — records,
coordination, communication — when you cannot use the main internet?

I've had the privilege of sitting with some of them. We worked together on
identity systems for a provisional government — which is where I learned,
concretely, why an identity system must not have a registry. In Myanmar,
being stopped at a checkpoint with the wrong ID can mean arrest. A
database of who people are, read backwards by the wrong hands, is a
weapon. When I said earlier that we never built the panopticon — that
design decision has names and faces attached to it.

But the first thing they taught me came before any of that. We kept
trying to start on identity, and the conversation kept returning to
something more basic: connectivity itself. Because in Myanmar,
disconnection is one of the weapons.

The military cuts the cell towers. Starlink is banned. ISPs are shut off,
region by region. An area is made to go dark, and then it is moved
through. And I heard what the dark is like from people who lived it: news
reduced to what someone can hand-copy onto paper and carry by motorbike,
hundreds of miles — which towns had burned, whether the soldiers were
coming toward you or turning somewhere else. Your village, and silence,
and waiting to find out.

Cutting people off from each other, to take them apart piece by piece, is
exactly the thing the internet was built to make impossible. That was the
founding requirement — survive the loss of your center. We spent forty
years giving it a head. 

a mesh node is not a shield against an army
The claim is narrower, but it's still worthy: **a network with no head to
cut off is harder to take away.**  Censorship-Resistance. 
No tower whose removal darkens a region
— the mesh rides whatever link exists: a radio, a wire, one shared
satellite hop. No ISP to ban, because joining *is* reachability. No
coordinator to seize, no registry to read backwards, and when the network
is split — not if, *when* — both halves keep working, because we built
split-brain to be boring.

Resilience. Connectivity is a human right — because being cut off from each other is one of the oldest
ways human beings are broken. A network worth building is one that cannot
be taken away.

*[Hold one quiet beat. Then — the turn, into resolve.]*

---

## Beat 11 — Locked open (~4 min)

*[Slide: the principles return. The seventh word lights up: **un-ownable —
locked open.** Energy returns to the voice here — resolve, then invitation.]*

So here's the last word on the list, the one the title of this talk comes
from.

Everything I've shown you tonight serves one design goal, and it was never
a technical one. **Make it fundamentally not ownable.** Locked open — and I
mean that phrase precisely. Not open like a product, where
somebody chose to open it and somebody could choose to close it again. Open
the way a mathematical fact is incontrovertibly open. There is no center to buy. No center
to seize. No center to subpoena, pressure, acquire, or slowly bend toward
extraction — because there is no center at all, and nothing in the design
allows one to form. The shortcut of authority always leaves a SPOF, and
history says someone eventually captures it. So we did the harder engineering,
all the way down, to build a thing with no center at all.

That's my definition of the decentralized web, for whatever it's worth —
not centralized services with the logo filed off and a token bolted on,
but systems whose **correctness does not route through anyone's
authority.**

*[Beat. Now hand it over.]*

And the method is yours. Three questions, for whatever you're building or
using or depending on: 
- Who mints your identity? 
- Are you a peer, or a customer? 
- Who decides whose version wins? 

Wherever the answer is "an authority" — that's not a fact of nature. It's a design decision, and it
can be redesigned. Every "granted" you find can become something created,
claimed, converged. It takes real engineering — and what it buys is the only guarantee that
survives every owner, every acquisition, every regime: there's nothing
there to take.

Because the ways we coordinate — the group chat, the shared document, the
directory, the map of which towns are safe — this is necessary
infrastructure now, as necessary as water. And necessary infrastructure
cannot live in the hands of anyone whose incentive is to own it. Build it
so it can't be.

*[Beat. One more admission — quieter, then done.]*

And one thing about ownership, since the whole talk is about not having any.
This project used to belong to my company. As of this week, it doesn't. I
signed it over to a foundation — the World Tree Network Foundation — that
exists to hold it and nothing else. Same reason as everything else tonight:
a thing that's locked open can't have *me* as its throat either. No center
to seize, and now no owner to pressure.

A network is created by the people who join it.
That was true in this room tonight, and there's no reason it can't be true
everywhere.

Come find me — the mesh is an invitation, and it merges.

Thank you.

*[Slide: title. Then Q&A.]*

