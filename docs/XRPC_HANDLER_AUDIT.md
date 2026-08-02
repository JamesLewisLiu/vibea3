# Core XRPC handler compatibility audit

This audit separates facts visible in the official clients from server policy.
`avs2-ea3.dll` and `popn.dll` prove wire layouts, required response fields, and
client status handling. Database decisions are compared with the copied
Mermaid server implementation.

| Route | Official client contract and verified policy |
| --- | --- |
| `services.get` | Optional `info/AVS2`; response `expire`, `mode`, `product_domain`, and repeated `item` with mandatory `name@` and `url@`. Core always advertises the AVS-EA3 built-ins plus `ntp` and `keepalive`; matching dynamic modules contribute their declared game-specific services. Modern XRPC URLs use the declaring module ID (`/core-services`, `/popn-highcheers`) rather than the individual service name. Legacy XRPC service URLs use `/+`; `ntp` and `keepalive` retain their dedicated configured URLs. |
| `pcbtracker.alive` | Request supports `model@`, `hardid@`, `softid@`, `accountid@`, `agree@`, and `ecflag@`. Client consumes `expire@`, `time@`, `limit@`, `ecenable@`, and `eclimit@`. Official clients understand status `127` as a contract prompt and LMA uses eacoin flag `3`; Vibea3 deliberately does not implement Mermaid's PCB registration/contract gate and returns success without requiring a machine record. |
| `message.get` | Response `expire`; every emitted item must contain `name@`, signed `start@`, and signed `end@`. Maintenance emits `sys.mainte` and `sys.eacoin.mainte`. |
| `facility.get` | Client retains the complete facility property tree and separately reads signed `calendar/year` and up to 62 signed 16-bit holidays. Public coordinates use `N35.41.22.2`/`E139.41.29.9`-style DMS text. Missing PCB data returns status `1`. |
| `package.list` | `secondary@` is optional. Package items are optional; an empty list with `secondary=0` is valid. |
| `pcbevent.put` | Request always contains `time` (`time`), `seq` (`u32`), and repeated `item` nodes containing `name` (`str`), `value` (`s32`), and `time` (`time`). Empty responses are accepted. Vibea3 acknowledges these packets without MongoDB persistence; optional event storage belongs in a Redis service. |
| `eacoin.checkin` | Request nodes are `cardtype`, `cardid`, `passwd`, and `ectype`. On status `0`, the client requires `sequence:s16`, `acstatus:u8`, `acid:str`, `acname:str`, `balance:s32`, `sessid:str`, `inshopcharge:u8`, and `point:s32`. |
| `eacoin.checkout` | Request contains `sessid`; the client closes its local session from the RPC result and does not require response payload fields. |
| `eacoin.consume` | Request has `esdate@`, `esid@`, `sessid:str`, `sequence:s16`, `payment:s32`, `service:s32`, `itemtype:str`, and `detail:str`. A successful RPC requires `acstatus:u8`, `autocharge:u8`, and `balance:s32`; insufficient balance is status `0` with `acstatus=1`. |
| `system*.convcardnumber` | Request `data/card_id`; success is `result=0` plus `data/card_number`; invalid input is RPC status `1` with `result=1`. All three route names share the same conversion algorithm. |
| `cardmng.getrefid` | Despite its name, this is the card-registration endpoint. Request attributes are `cardid`, `cardtype`, `newflag`, and `passwd`; success supplies an authenticated session `refid` and global-user `dataid`. A previously unseen valid card creates both its card record and a user record, while an existing inactive card is activated. Active cards are always rejected with status `1` and must use `inquire` followed by `authpass`, so registration cannot reset an existing PIN. Invalid card/PIN is also status `1`. |
| `cardmng.inquire` | Request attributes `cardid`, `cardtype`, and `update`. Pop'n consumes `refid`, `dataid`, `binded`, `expired`, and `ecflag`. Active cards return status `0` and their owning user's `dataid`; multiple cards may share that user. Missing and inactive cards return `112`, which popn.dll maps to its new-card registration branch. Banned is `109`. Only the all-zero card-reader sentinel returns status `0` with dummy IDs. |
| `cardmng.authpass` | Request attributes `refid` and `pass`; invalid PIN is status `116`, missing session is `1`, success is `0`. Pop'n's status converter explicitly distinguishes `109`, `112`, and `116`. |
| `cardmng.bindmodel` | Request attribute `refid`; a valid authenticated session returns status `0` with its global-user `dataid`. Invalid, expired, or unauthenticated sessions return `1`. |
| `cardmng.getdatalist` | Empty request and empty status-`0` response. The method is part of the five-method AVS-EA3 card manager contract even when no legacy data list is configured. |
| `dlstatus.progress` | Pop'n sends a large PCB download-status structure, but only the module RPC status is required from the response. Unknown request nodes are intentionally ignored by the compact server schema. |

## Shared response quirks

- Normal responses copy request `srcid` to root `dstid`.
- `services.get` has no product-letter-based "basic" catalog. It always emits
  the core entries and merges only services exported by modules matching the
  caller's model and datecode.
- The unconditional XRPC entries are `services`, `pcbtracker`, `pcbevent`,
  `message`, `facility`, `apsmanager`, `sidmgr`, `cardmng`, `package`,
  `dlstatus`, `eacoin`, and `ins`. `ntp` and `keepalive` are also unconditional
  transport endpoints and use their own configured URLs.
- Nine-character model strings such as `FDD:J:A:A` omit `fault="0"`; those
  clients interpret a zero-valued fault attribute as an error.
- Handler schemas tolerate additional official-client request fields when the
  server policy does not use them, while still enforcing every field the
  official client requires in successful responses.
