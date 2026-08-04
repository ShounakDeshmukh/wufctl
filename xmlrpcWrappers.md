## Observed access against NCSU's live VCL server (2026-08-03)

Verified live against `vcl.ncsu.edu` using a standard (non-admin) NCSU
account and vcl_lib's `vcl_probe` example crate. Confirmed working
end-to-end: `XMLRPCtest`, `XMLRPCgetImages`, `XMLRPCgetRequestIds`,
`XMLRPCaddRequest`, `XMLRPCgetRequestStatus`, `XMLRPCgetRequestConnectData`,
`XMLRPCextendRequest`, `XMLRPCendRequest`, `XMLRPCgetUserGroups`. See
per-method notes below for the methods that failed and why. Everything
else in this document (privilege tree writes, user group writes, group
membership, `XMLRPCautoCapture`) was not exercised against the live
server - either because it mutates shared/institutional state rather than
the caller's own reservations, or because no real identifier to test with
could be discovered from this account's read-only access.

One data-shape note for client implementers: this server encodes some
nominally-numeric fields (e.g. `requestid` in `XMLRPCaddRequest`'s
response, several fields in `XMLRPCgetUserGroups`' response) as XML-RPC
`<string>` rather than `<int>`. Don't assume numeric response fields are
typed as `<int>`.

### `XMLRPCaffiliations()`

* **Description:** Gets all of the affiliations for which users can log in to VCL.
* **Parameters:** None
* **Note:** This is the only function available for which the `X-User` and `X-Pass` HTTP headers do not need to be passed.
* **Observed:** Not exposed on NCSU's server - returns XML-RPC fault `-32601 method not found`.

### `XMLRPCtest($string)`

* **Description:** A test function that can be called when getting XML RPC calls to this site to work.
* **Parameters:**
* `$string`: A string


* **Returns:** An array with 3 indices:
* `status` - will be 'success'
* `message` - will be 'RPC call worked successfully'
* `string` - contents of `$string` (after being sanitized)



### `XMLRPCgetImages()`

* **Description:** Gets the images to which the user has access.
* **Parameters:** None

### `XMLRPCaddRequest($imageid, $start, $length, $foruser='', $nousercheck=0)`

* **Description:** Tries to make a request.
* **Parameters:**
* `$imageid`: The ID of the image.
* `$start`: The start time.
* `$length`: The duration length.
* `$foruser` (default `''`): The user the request is for (String).
* `$nousercheck` (default `0`): Flag to skip user check (Integer).



### `XMLRPCaddRequestWithEnding($imageid, $start, $end, $foruser='', $nousercheck=0)`

* **Description:** Tries to make a request with the specified ending time.
* **Parameters:**
* `$imageid`: The ID of the image.
* `$start`: The start time.
* `$end`: The ending time.
* `$foruser` (default `''`): The user the request is for (String).
* `$nousercheck` (default `0`): Flag to skip user check (Integer).
* **Observed:** Access denied for this account when an explicit `$end` is given - fault `errorcode 35 "access denied to specify end time"`. Same restriction as `XMLRPCsetRequestEnding` below. No reservation is created when this fires.



### `XMLRPCdeployServer($imageid, $start, $end, $admingroup='', $logingroup='', $ipaddr='', $macaddr='', $monitored=0, $foruser='', $name='', $userdata='')`

* **Description:** Tries to make a server request.
* **Parameters:**
* `$imageid`: The ID of the image.
* `$start`: The start time.
* `$end`: The ending time.
* `$admingroup` (default `''`): Admin group string.
* `$logingroup` (default `''`): Login group string.
* `$ipaddr` (default `''`): IP address string.
* `$macaddr` (default `''`): MAC address string.
* `$monitored` (default `0`): Monitored flag (Integer).
* `$foruser` (default `''`): User string.
* `$name` (default `''`): Name string.
* `$userdata` (default `''`): User data string.
* **Observed:** Access denied for this account - fault `errorcode 60 "access denied to deploy server"`. No reservation is created when this fires.



### `XMLRPCgetRequestIds()`

* **Description:** Gets information about all of user's requests.
* **Parameters:** None

### `XMLRPCgetRequestStatus($requestid)`

* **Description:** Determines and returns the status of the request.
* **Parameters:**
* `$requestid`: The ID of the request.



### `XMLRPCgetRequestConnectData($requestid, $remoteIP)`

* **Description:** If the request is ready, adds the connecting user's computer to the request and returns info about how to connect to the computer.
* **Parameters:**
* `$requestid`: The ID of the request.
* `$remoteIP`: The remote IP address.



### `XMLRPCextendRequest($requestid, $extendtime)`

* **Description:** Extends the length of an active request; if a request that has not started needs to be extended, delete the request and submit a new one.
* **Parameters:**
* `$requestid`: The ID of the request.
* `$extendtime`: The time to extend the request by.



### `XMLRPCsetRequestEnding($requestid, $end)`

* **Description:** Modifies the end time of an active request; if a request that has not started needs to be modified, delete the request and submit a new one.
* **Parameters:**
* `$requestid`: The id of a request.
* `$end`: Unix timestamp for end of reservation; will be rounded up to the nearest 15 minute increment.


* **Returns:** An array with at least one index named 'status' which will have one of these values:
* `error` - error occurred; there will be 2 additional elements in the array: `errorcode` (error number) and `errormsg` (error string).
* `success` - request was successfully extended.
* **Observed:** Access denied for this account - fault `errorcode 35 "access denied to specify end time"`.



### `XMLRPCendRequest($requestid)`

* **Description:** Ends/deletes a request.
* **Parameters:**
* `$requestid`: The ID of the request to end.



### `XMLRPCautoCapture($requestid)`

* **Description:** Creates entries in appropriate tables to capture an image and sets the request state to image.
* **Parameters:**
* `$requestid`: The ID of the request.



### `XMLRPCgetGroupImages($name)`

* **Description:** Gets a list of all images in a particular group.
* **Parameters:**
* `$name`: The name of the group.



### `XMLRPCaddImageToGroup($name, $imageid)`

* **Description:** Adds an image to a resource group.
* **Parameters:**
* `$name`: The name of the group.
* `$imageid`: The ID of the image.



### `XMLRPCremoveImageFromGroup($name, $imageid)`

* **Description:** Removes an image from a resource group.
* **Parameters:**
* `$name`: The name of the group.
* `$imageid`: The ID of the image.



### `XMLRPCaddImageGroupToComputerGroup($imageGroup, $computerGroup)`

* **Description:** Map an image group to a computer group.
* **Parameters:**
* `$imageGroup`: The image group.
* `$computerGroup`: The computer group.



### `XMLRPCremoveImageGroupFromComputerGroup($imageGroup, $computerGroup)`

* **Description:** Remove the mapping of an image group to a computer group.
* **Parameters:**
* `$imageGroup`: The image group.
* `$computerGroup`: The computer group.



### `XMLRPCgetNodes($root=NULL)`

* **Description:** Gets a list of all nodes in the privilege tree.
* **Parameters:**
* `$root` (default `NULL`): The root node.
* **Observed:** Access denied for this account - fault `errorcode 70 "User cannot access node content"`. Standard accounts likely can't reach any of the privilege-tree or resource-group-privilege methods below; not independently verified since there's no way to discover a real node/group identifier without this call working first.



### `XMLRPCnodeExists($nodeName, $parentNode)`

* **Description:** Indicates whether a node with that name already exists at this location in the privilege tree.
* **Parameters:**
* `$nodeName`: The name of the node.
* `$parentNode`: The parent node.



### `XMLRPCaddNode($nodeName, $parentNode)`

* **Description:** Add a node to the privilege tree as a child of the specified parent node.
* **Parameters:**
* `$nodeName`: The name of the new node.
* `$parentNode`: The parent node to attach to.



### `XMLRPCremoveNode($nodeID)`

* **Description:** Delete a node from the privilege tree.
* **Parameters:**
* `$nodeID`: The ID of the node to remove.



### `XMLRPCgetUserGroupPrivs($name, $affiliation, $nodeid)`

* **Description:** Get a list of privileges for a user group at a particular node in the privilege tree.
* **Parameters:**
* `$name`: The user group name.
* `$affiliation`: The affiliation.
* `$nodeid`: The node ID.



### `XMLRPCaddUserGroupPriv($name, $affiliation, $nodeid, $permissions)`

* **Description:** Add privileges for a user group at a particular node in the privilege tree.
* **Parameters:**
* `$name`: The user group name.
* `$affiliation`: The affiliation.
* `$nodeid`: The node ID.
* `$permissions`: The privileges to add.



### `XMLRPCremoveUserGroupPriv($name, $affiliation, $nodeid, $permissions)`

* **Description:** Remove privileges for a resource group at a particular node in the privilege tree.
* **Parameters:**
* `$name`: The user group name.
* `$affiliation`: The affiliation.
* `$nodeid`: The node ID.
* `$permissions`: The privileges to remove.



### `XMLRPCgetResourceGroupPrivs($name, $type, $nodeid)`

* **Description:** Get a list of privileges for a resource group at a particular node in the privilege tree.
* **Parameters:**
* `$name`: The resource group name.
* `$type`: The type of resource.
* `$nodeid`: The node ID.



### `XMLRPCaddResourceGroupPriv($name, $type, $nodeid, $permissions)`

* **Description:** Add privileges for a resource group at a particular node in the privilege tree.
* **Parameters:**
* `$name`: The resource group name.
* `$type`: The type of resource.
* `$nodeid`: The node ID.
* `$permissions`: The privileges to add.



### `XMLRPCremoveResourceGroupPriv($name, $type, $nodeid, $permissions)`

* **Description:** Remove privileges for a resource group from a node in the privilege tree.
* **Parameters:**
* `$name`: The resource group name.
* `$type`: The type of resource.
* `$nodeid`: The node ID.
* `$permissions`: The privileges to remove.



### `XMLRPCgetUserGroups($groupType=0, $affiliationid=0)`

* **Description:** Builds a list of user groups.
* **Parameters:**
* `$groupType` (default `0`): The group type (Integer).
* `$affiliationid` (default `0`): The affiliation ID (Integer).



### `XMLRPCgetUserGroupAttributes($name, $affiliation)`

* **Description:** Gets information about a user group.
* **Parameters:**
* `$name`: The user group name.
* `$affiliation`: The affiliation.



### `XMLRPCaddUserGroup($name, $affiliation, $owner, $managingGroup, $initialMaxTime, $totalMaxTime, $maxExtendTime, $custom=1)`

* **Description:** Creates a new user group with the specified parameters.
* **Parameters:**
* `$name`: The group name.
* `$affiliation`: The affiliation.
* `$owner`: The group owner.
* `$managingGroup`: The managing group.
* `$initialMaxTime`: The initial max time.
* `$totalMaxTime`: The total max time.
* `$maxExtendTime`: The max extension time.
* `$custom` (default `1`): Custom flag (Integer).



### `XMLRPCeditUserGroup($name, $affiliation, $newName, $newAffiliation, $newOwner='', $newManagingGroup='', $newInitialMaxTime='', $newTotalMaxTime='', $newMaxExtendTime='')`

* **Description:** Modifies attributes of a user group.
* **Note:** An empty string may be passed for any of the `new*` fields to leave that item unchanged.
* **Parameters:**
* `$name`: The group name.
* `$affiliation`: The affiliation.
* `$newName`: The new name.
* `$newAffiliation`: The new affiliation.
* `$newOwner` (default `''`): The new owner string.
* `$newManagingGroup` (default `''`): The new managing group string.
* `$newInitialMaxTime` (default `''`): The new initial max time string.
* `$newTotalMaxTime` (default `''`): The new total max time string.
* `$newMaxExtendTime` (default `''`): The new max extend time string.

## Methods available to the TUI's target audience

The TUI targets NCSU students authenticating with a standard (non-admin) VCL
account - the same population that currently uses the VCL web UI, not VCL
administrators. This section filters the full method list above down to
what that audience can actually call, based on live testing against
`vcl.ncsu.edu` (see "Observed access" at the top of this document).

### Usable today - build the TUI on these

The full create -> monitor -> connect -> extend -> end lifecycle, confirmed
working end-to-end:

* `XMLRPCtest($string)` - connectivity/auth check.
* `XMLRPCgetImages()` - list images available to reserve.
* `XMLRPCaddRequest($imageid, $start, $length, $foruser='', $nousercheck=0)` - create a reservation.
* `XMLRPCgetRequestIds()` - list your own requests.
* `XMLRPCgetRequestStatus($requestid)` - poll a reservation's status.
* `XMLRPCgetRequestConnectData($requestid, $remoteIP)` - get connect info once ready.
* `XMLRPCextendRequest($requestid, $extendtime)` - extend an active reservation.
* `XMLRPCendRequest($requestid)` - end/delete a reservation.
* `XMLRPCgetUserGroups($groupType=0, $affiliationid=0)` - read-only; useful for context, not group management.

`vcl_lib` also wraps one undocumented method, `XMLRPCgetIP`, used to fill in
`$remoteIP` for `XMLRPCgetRequestConnectData` automatically - not part of the
official spec, but confirmed working and safe to build on.

### Confirmed blocked for this account type - don't build UI around these

* `XMLRPCaffiliations()` - not exposed on this server at all (fault `-32601 method not found`).
* `XMLRPCaddRequestWithEnding(...)` - access denied when an explicit end time is given (errorcode 35).
* `XMLRPCdeployServer(...)` - access denied (errorcode 60); server deployment is an admin-tier feature.
* `XMLRPCsetRequestEnding(...)` - access denied, same errorcode 35 as above.
* `XMLRPCgetNodes(...)` - access denied (errorcode 70); privilege-tree browsing is admin-only.

### Untested, but administrative in nature - assume unavailable

Everything else in this document operates on shared/institutional state
(privilege tree, resource groups, group membership) or has no undo path
(`XMLRPCautoCapture`). None of it was exercised against the live server, but
given the access-denied pattern above, a standard student account almost
certainly can't reach it either:

`XMLRPCautoCapture`, `XMLRPCgetGroupImages`, `XMLRPCaddImageToGroup`,
`XMLRPCremoveImageFromGroup`, `XMLRPCaddImageGroupToComputerGroup`,
`XMLRPCremoveImageGroupFromComputerGroup`, `XMLRPCnodeExists`,
`XMLRPCaddNode`, `XMLRPCremoveNode`, `XMLRPCgetUserGroupPrivs`,
`XMLRPCaddUserGroupPriv`, `XMLRPCremoveUserGroupPriv`,
`XMLRPCgetResourceGroupPrivs`, `XMLRPCaddResourceGroupPriv`,
`XMLRPCremoveResourceGroupPriv`, `XMLRPCgetUserGroupAttributes`,
`XMLRPCaddUserGroup`, `XMLRPCeditUserGroup`.

### Practical implication for the TUI

Build the TUI around the 9 confirmed-working methods (plus `get_ip` for
convenience) only: browse images, create a request, watch it move through
statuses, connect once ready, optionally extend, and end it when done. Skip
any UI affordance for scheduling an exact end time, server deployment, or
group/privilege management - none of it will work for the account type this
tool targets.