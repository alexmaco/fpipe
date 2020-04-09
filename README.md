# fpipe

Filter (and map) in a shell pipe.

## Usage

Only list files that contain cats:

`ls | fpipe --silence grep cats {}`

Only keep files that do **not** contain cats:

`cat index | fpipe -n -s grep cats {}`

## Installation

`cargo install -f --git https://github.com/alexmaco/fpipe`
