# VCL XML-RPC response shapes

Field-level spec for the response deserialization layer living in `vcl_tui`
(the TUI converts `vcl_lib::Value` into typed structs itself; `vcl_lib` keeps
returning the untyped value). Scoped to the 9 "usable today" methods listed
in `../xmlrpcWrappers.md` for a standard NCSU account, plus `get_ip`.

Captured 2026-08-08 by running `vcl_probe` against a live account on
`vcl.ncsu.edu`. Where a shape wasn't actually observed on the wire (the test
reservation never left `loading`/`notready` in the probe's short run), it's
marked **unconfirmed** below rather than guessed from the source PHP docs.

## Quirks to design around

- Numeric fields are inconsistently typed by the server: `<int>` in some
  responses (`get_images` id, `get_request_status` time, error codes),
  `<string>` in others (`get_user_groups` ids/times, `add_request`'s
  `requestid`). Deserialization must accept both `Value::Int` and a
  parseable `Value::String` and coerce to the target numeric type.
- `Value::Struct` is a `HashMap` - field order is not stable, don't rely on
  it.
- A `status` field appears on nearly every response and doubles as a
  success/error discriminant, but its value vocabulary is method-specific
  (see below) - it isn't a single global enum.
- Field naming for `get_request_connect_data` may not be fully stable
  across VCL deployments/versions: a community CLI targeting the same API
  (github.com/NisargJasani0602/VCL, see `vcl/commands.py`) defensively
  checks several key-name variants for the same logical field -
  `serverIP`/`serverip`, `user`/`userid`, `connectport`/`port`,
  `password`/`passwd`/`reservationpassword`/`connectpassword`. Our live
  probe only ever saw `serverIP`/`user`/`connectport`/`password` (see
  below), which is what this spec documents, but that hedging is a signal
  worth keeping in mind if deserialization ever hits an unexpected
  variant on a different VCL instance.

## Per-method shapes

### `test(message)` - `XMLRPCtest`

```
Struct {
    status:  String  ("success")
    message: String  ("RPC call worked successfully")
    string:  String  (echo of the input, server-sanitized)
}
```

### `get_images()` - `XMLRPCgetImages`

Returns `Array<Struct>`, one entry per image:

```
Struct {
    id:          Int
    name:        String
    ostype:      String   ("linux" | "windows" observed)
    usage:       String   (free text, often "")
    description: String   (free text, may contain HTML like "<br>")
}
```

### `get_request_ids()` - `XMLRPCgetRequestIds`

```
Struct {
    status:   String        ("success")
    requests: Array<Struct> (empty in this run - no active requests existed,
                              so the element shape is UNCONFIRMED)
}
```

### `get_user_groups(type, affiliation)` - `XMLRPCgetUserGroups`

```
Struct {
    status: String        ("success")
    groups: Array<Struct>
}
```

Each group element - every field observed as `String`, including the
numeric-looking ones:

```
Struct {
    id:                      String  (numeric, e.g. "27")
    name:                    String  (e.g. "csc_grad@NCSU")
    affiliation:             String  (empty "" in every row this run)
    owner:                   String  (empty "" in every row this run)
    ownerid:                 String  (empty "" in every row this run)
    custom:                  String  ("0" | "1")
    courseroll:              String  (numeric)
    initialmaxtime:          String  (numeric, minutes)
    totalmaxtime:            String  (numeric, minutes)
    maxextendtime:           String  (numeric, minutes)
    overlapResCount:         String  (numeric)
    groupaffiliation:        String  (e.g. "NCSU")
    groupaffiliationid:      String  (numeric)
    editgroup:               String  (empty "" in every row this run)
    editgroupid:             String  (empty "" in every row this run)
    editgroupaffiliation:    String  (empty "" in every row this run)
    editgroupaffiliationid:  String  (empty "" in every row this run)
}
```

Whether `owner`/`editgroup*` fields are ever populated (vs. always empty for
groups this account can see) is UNCONFIRMED.

### `get_ip()` - `XMLRPCgetIP` (undocumented, not in the official spec)

```
Struct {
    status: String  ("success")
    ip:     String  (dotted-quad)
}
```

### `add_request(image_id, start, length, for_user, no_user_check)` - `XMLRPCaddRequest`

Success:

```
Struct {
    status:    String  ("success")
    requestid: String  (numeric, e.g. "4214683" - NOT Int despite being an id)
}
```

Error (shape confirmed live via the sibling `add_request_with_ending` /
`deploy_server` calls, which hit the same error path):

```
Struct {
    status:    String  ("error")
    errorcode: Int
    errormsg:  String
}
```

### `get_request_status(request_id)` - `XMLRPCgetRequestStatus`

Two states confirmed live, by polling every 35s (dev interval - see
`vcl_probe`'s `POLL_INTERVAL`; the production TUI should poll every 20s to
match the VCL web UI) until the reservation left `loading`:

`loading`:

```
Struct {
    status: String  ("loading")
    time:   Int      (meaning UNCONFIRMED - observed value 1 on every poll)
}
```

`ready`:

```
Struct {
    status: String  ("ready")
}
```

Note `time` is present in `loading` but absent in `ready` - it's not a
field that's always there, deserialization must treat it as optional.
UNCONFIRMED: any other status value (e.g. a `deleted`/expired state), and
whether `imageid`/`prettyimage` fields ever appear as `vcl_lib`'s doc
comment claims - not seen in either state observed so far.

### `get_request_connect_data(request_id, remote_ip)` - `XMLRPCgetRequestConnectData`

Two states confirmed live:

`notready`:

```
Struct {
    status: String  ("notready")
}
```

`ready` - notably different from `vcl_lib`'s doc comment (which claims
`ip`/`hostname`/`password`/`portNumber`); the field names below are what
the live server actually sends:

```
Struct {
    status:         String  ("ready")
    serverIP:       String  (dotted-quad, NOT "ip")
    user:           String  (username, e.g. "sdeshmu4" - undocumented field)
    password:       String  (for campus-auth accounts this is literally the
                              placeholder text "(use your campus password)",
                              not a real per-reservation secret - don't
                              assume it's always a usable credential)
    connectport:    String  (numeric string, e.g. "22" - NOT "portNumber")
    connectMethods: Struct<String, Struct> (keyed by a numeric-string method
                              id, e.g. "1", "6" - see below)
}
```

Each `connectMethods` entry:

```
Struct {
    description:  String  (e.g. "SSH (Secure Shell) on Port 22")
    connecttext:  String  (long HTML instructional text with literal `\r\n`/`\n`
                            and inline `<b>`/`<UL>`/`<a href>` markup - render
                            as HTML or strip tags, don't show raw)
    connectports: Array<String>  (e.g. ["TCP:22:22"] - "proto:local:remote")
}
```

Two methods were observed for this Linux image: `"1"` (SSH) and `"6"`
(xRDP). The key set and count are presumably image/ostype-dependent -
UNCONFIRMED for a Windows/AVD image.

### `extend_request(request_id, minutes)` - `XMLRPCextendRequest`

Success:

```
Struct {
    status: String  ("success")
}
```

Error (shape confirmed live via the sibling `set_request_ending` call):

```
Struct {
    status:    String  ("error")
    errorcode: Int
    errormsg:  String
}
```

### `end_request(request_id)` - `XMLRPCendRequest`

```
Struct {
    status: String  ("success")
}
```

## Follow-ups needed to close the remaining unconfirmed gaps

- `get_request_ids()` element shape: rerun while at least one request is
  already active (e.g. don't end the reservation from the lifecycle test
  before this call runs).
- `get_user_groups` `owner`/`editgroup*` populated case: would need a group
  this account owns or has edit rights on; none were found in this run.
- `get_request_connect_data`'s `connectMethods` shape for a non-Linux (AVD)
  image - only SSH/xRDP on a Linux image were observed.
- Any `get_request_status` value other than `loading`/`ready` (e.g. an
  expired/deleted reservation).
