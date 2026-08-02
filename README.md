# vibea3

Vibea3 is a typed Rust implementation of the e-Amusement HTTP RPC transport. It decodes XML or kbinxml directly into structs, dispatches `{class}.{method}` functions, and mirrors the request packet format, text encoding, custom LZ77 compression, and RC4 transport wrapper.

The serializer uses derive-generated static schemas and streams fields into typed builders. It does not construct an XML DOM. The decoder executes the prebuilt schema operations as a small VM.

## Workspace

```text
bootstrap/                 executable service host
modules/
  core/                    codec, schema VM, LZ77, transport, RPC and loader
  core/src/host.rs         schema-free host services and document-store contract
  core_module/             reloadable service module, models, queries, and indexes
    src/card_manage.rs
    src/pcb_tracker.rs
    src/lib.rs
  popn_highcheers/         M39 High Cheers game module and MongoDB models
  test_module/             reload/lifecycle fixture used only by tests
vibea3-macros/            Kbin and RPC procedural macros
```

`bootstrap` is the executable. `modules/core` remains a library because both the executable and every dynamic module must compile against the same RPC and ABI types.

## Dynamic handlers

Handlers register themselves in a link-time distributed slice. They can be spread across files and do not need to be repeated in `export_xrpc_module!`:

```rust
#[rpc("cardmng.inquire")]
async fn inquire(
    ctx: RpcContext<State>,
    request: InquireRequest,
) -> RpcResult<InquireResponse> {
    Ok(InquireResponse { /* ... */ })
}
```

One DLL or shared object may own any number of complete classes. Class ownership is selected by the caller product and datecode, so multiple modules may provide the same class for disjoint compatibility ranges. Export also declares service discovery metadata:

```rust
export_xrpc_module! {
    module: "core-services",
    version: "1.0.0",
    model: "LDJ",
    datecode_min: "20251200",
    datecode_max: "20261200",
    services: ["game", "musicdb"],
    host: dyn vibea3::HostServices,
    host_fingerprint: vibea3::HOST_API_FINGERPRINT,
    state: State,
    init: init,
    shutdown: shutdown,
}
```

`model`, `datecode_min`, `datecode_max`, and `services` are optional. Datecode bounds are inclusive eight-digit values; callers with longer build datecodes use the first eight digits. A bounded module does not receive calls with a missing or malformed datecode. Modules with the same class may coexist when their product codes differ or their datecode ranges do not overlap. `services.get` always includes the AVS-EA3 core entries, `ntp`, and `keepalive`, then merges and de-duplicates the `services` names from every module matching the caller model. Each XRPC service URL uses its declaring module ID as the endpoint path, so core services share `/core-services` and Pop'n services share `/popn-highcheers`.

`init` receives `Arc<dyn HostServices>` and returns fresh cloneable state. An optional `shutdown` entry receives a cloned state after the old generation drains. Lifecycle functions return `Result<_, E>` where `E: ToString`.

`RpcContext::call()` performs typed in-memory calls to another registered route using full-name UTF-8 kbin, without HTTP, RC4, or LZ77. Internal calls have a depth limit of 16.

## Reload behavior

- Top-level `.dll` files on Windows and `.so` files on Linux are discovered automatically.
- Candidates are checked for a stable size and modification time, hashed, and shadow-copied before loading.
- ABI version, Rust toolchain/target/profile fingerprint, host type, and host API fingerprint are validated before initialization.
- A replacement generation is published atomically for all of its classes.
- Existing calls pin their old library and state until their futures complete; new calls use the new generation.
- Invalid binaries, initialization failures, route conflicts, and panics leave the last good generation active.
- Deleting a source file marks the module missing but keeps its last good generation until explicit unload.

Plugins and the host must use the same Rust toolchain, target, profile, Vibea3 version, ABI-relevant features, and shared service API. Modules are trusted in-process code, not a sandbox. Runtime-bound APIs such as Tokio timers should be exposed through `HostServices`; a separately linked DLL has separate runtime thread-local state.

## Administration

The public router exposes bearer-token-protected JSON endpoints:

```text
GET  /_vibea3/admin/v1/modules
POST /_vibea3/admin/v1/modules/rescan
POST /_vibea3/admin/v1/modules/{module_id}/reload
POST /_vibea3/admin/v1/modules/{module_id}/unload
```

Responses use `Cache-Control: no-store`. The token is compared in constant time.

## Running the server

In a source checkout, run the complete development server with:

```powershell
$env:VIBEA3_ADMIN_TOKEN = "change-me"
$env:VIBEA3_DATA_DIR = (Resolve-Path data)
cargo run
```

In debug builds the bootstrap discovers every workspace `cdylib` marked with `[package.metadata.vibea3] module = true`, builds them together, and copies their DLLs/shared objects into `VIBEA3_MODULE_DIR` before connecting to MongoDB. Test fixtures are not deployed. Set `VIBEA3_BUILD_MODULES=false` to skip this development step. Release binaries skip it by default, or it can be explicitly enabled for a source checkout. `VIBEA3_LISTEN` defaults to `127.0.0.1:5000`, and `VIBEA3_MODULE_DIR` defaults to `modules-bin`.

The bootstrap loads the first `.env` found from the working directory upward without overwriting variables already present in the process environment. Copy `.env.example` to `.env` for local development. `RUST_LOG` controls target filters, and `VIBEA3_LOG_FORMAT` selects `compact`, `pretty`, or newline-delimited `json` output. HTTP request status/latency and dynamic module lifecycle events are logged without request bodies, authorization headers, or database credentials.

Decoded input and output packets can be logged as normalized UTF-8 XML without changing the wire format:

```text
RUST_LOG=vibea3_server=info,vibea3=info,vibea3::rpc::packet=debug,tower_http=info
```

The packet target also records the route, model, source ID, tag, peer address, internal-call depth, RPC status, packet format, compression, encryption state, and encoded/decoded sizes. Packet debug logging may contain card IDs, PINs, player data, and other private request content, so it should only be enabled while diagnosing traffic.

`popn_highcheers` requires `popn_highcheers/info.json` and `popn_highcheers/music.json` under `VIBEA3_DATA_DIR`, or their containing directory may be supplied directly with `POPN_HIGHCHEERS_DATA_DIR`. Both files are validated once when the DLL is loaded; a bad hot-update is rejected while the previous generation remains active.

The executable owns the MongoDB connection, generic document operations, and GeoIP connection so reloadable modules do not create a second Tokio runtime boundary. It has no knowledge of service collections or document shapes. Each module owns its database models, queries, and indexes. MongoDB defaults to `mongodb://172.29.14.146:27017`, database `vibea3`. Override those with `MONGODB_URI` and `MONGODB_DATABASE`.

Seed a development machine and card with:

```powershell
mongosh "mongodb://172.29.14.146:27017/vibea3" modules/core_module/mongodb-init.js
```

Set `GEOIP_DATABASE` to a MaxMind City `.mmdb` file to derive the facility country, region, latitude, and longitude from the request peer address. `facility.get/portfw/globalip` is always the client's TCP IPv4 address; `VIBEA3_PUBLIC_IP` is only the configured server-side fallback when no usable IPv4 peer address exists.

The production module implements:

```text
services.get             pcbtracker.alive       message.get
facility.get             package.list           pcbevent.put
eacoin.checkin           eacoin.checkout         eacoin.consume
system.convcardnumber    system_2.convcardnumber system_3.convcardnumber
cardmng.getrefid         cardmng.inquire         cardmng.authpass
cardmng.bindmodel        cardmng.getdatalist
dlstatus.progress
```

The core module persists global users, cards, short-lived card sessions, e-amusement coin ledger entries, and optional per-cabinet facility overrides. `dataid` is the permanent global user ID; cards reference it through `cards.user_id`, so multiple cards can share one account. Session `refid` values expire after 24 hours through a MongoDB TTL index and legacy session rows without expiry metadata are removed at startup. A machine document is never required to use the service. The module creates its own indexes during initialization. `pcbevent.put` is acknowledged without MongoDB persistence; event storage belongs in a separate Redis-backed service if enabled later.

E-amusement coin is unlimited by default: check-in and consume return a fixed non-decreasing balance without writing a ledger entry. Set `VIBEA3_INFINITE_EACOIN=false` to enable MongoDB-backed finite card balances and ledger records.

## Packet fields

Every `Vec<T>` declares its wire representation explicitly: use `#[kbin(array)]` for one typed `__type`/`__count` node, or `#[kbin(repeated)]` for repeated XML-style child nodes.

```rust
#[derive(Kbin)]
#[kbin(node = "player")]
struct Player {
    #[kbin(attr)]
    refid: String,
    #[kbin(array)]
    scores: Vec<i32>,
    position: [f32; 3],
}
```

Derived serializers sort user attribute field names lexicographically at compile time, matching the official library. Child nodes deliberately retain schema/declaration order, which also matches the official behavior. Reserved typed metadata is emitted in the official fixed order: `__type`, then `__count`, then `__size` where applicable.

Run the full codec, Python interoperability, XML sample, transport, RPC, dynamic-library, rollback, and reload suite with `cargo test --workspace --all-targets`.
