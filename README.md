There are two parts to this system. The node itself and the "explorer". The node itself manages the data and connects to its neighbors. While the explorer gives a way to control the node and query its data.

1) clone this repo

2) To run the node use the command `cargo run --bin ddb_node`. When the node starts it will output its id, this is important in the next step.

3) Run the explorer with `cargo run --bin ddb_explorer`. The explorer can accept a variety of commands, the first of which is to set the id it should use when connecting. In the explorer run `id <id from step 2>` to set the id.

4) The explorer is not connected by default. Use `connect 127.0.0.1:2000` to connect to the node.

Operations

Set a value with `set <keyname> <value>`.

Get the most recent value with `get <keyname>`. Or get the most recent n values with `get <keyname> n`

However, a single node is not likely to be much value, to have the node connect to another node use `link <ipaddr>:<port>`. You may now see messages in the explorer terminal as messages are routed through the system.

To change trust in another node use `trust <node_id> <trust_change>`. Where `<trust_change>` is a positive or negative integer to indicate the change. The range of trust goes from 0 to 10,000 and starts in the middle at 5,000. `trust <node_id> 2600` should make that node trusted, while `trust <node_id> -2600` should be enough to make it distrusted.


Finally, the command `disconnect` will disconnect the explorer from the node. And `quit` will exit the explorer.