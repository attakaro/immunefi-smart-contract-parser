Immunefi smart contract parser that parses the solidity source code of smart contracts of immunefi bug bounty projects.

Add your api keys to "keys.json" file manually:

```json
"chain name": [
    "your key", 
    "your api url"
]
```

Or use:

```bash
./iscp add_api <name> <key> <api_url>
```

**All urls must start with https://**.

**The chain name must be a substring of the smart contract url, e.g. "etherscan" name, "https://etherscan.io/0xFFFFFFF" url**.


Parse single contract using direct url:

```bash
./iscp parse <smart contract url>
```

Parse contracts from immunefi using immunefi bounty link:

```bash
./iscp parse_imm <immunefi bounty url> <folder name> <api requests at the same time> 
```
If you have free api plan, set api requests to 2.

To see more info about commands use:

```bash
./iscp help <command>
```
