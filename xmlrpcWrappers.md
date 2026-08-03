### `XMLRPCaffiliations()`

* **Description:** Gets all of the affiliations for which users can log in to VCL.
* **Parameters:** None
* **Note:** This is the only function available for which the `X-User` and `X-Pass` HTTP headers do not need to be passed.

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