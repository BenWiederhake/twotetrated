#!/bin/bash

set -exo pipefail

cd -- "$(dirname -- "$0")"

grep -Firn '// GRAMMAR: ' src/ \
 | sed -Ee 's,.*// GRAMMAR: ,,' \
 > doc/language_grammar.txt

# On the one hand, this probably belongs into build.rs.
# On the other hand, this is such a simple script that it feels like a crime to make it any more complicated than that.
