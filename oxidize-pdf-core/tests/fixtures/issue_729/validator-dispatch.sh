#!/bin/sh
# The scenario is data; only this immutable fixture is executed directly.
exec /bin/sh "${0%/*}/scenario.sh" "$@"
