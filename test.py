#!/usr/bin/env python3

import sqlite3

conn = sqlite3.connect("test.db", isolation_level=None)

print(f"Loading SQLite extension in connection: {conn}")
conn.enable_load_extension(True)
conn.execute(
    "SELECT load_extension('target/release/libsqlite_regex_ext.dylib', 'sqlite3_regex_init');"
)

print("Running tests...")

print("Testing pattern 'x(ab)' WITHOUT capture group")
row = conn.execute("SELECT regex_extract('x(ab)', 'xxabaa')").fetchone()
assert row[0] == "xab", row[0]

print("Testing pattern 'x(ab)' WITH capture group = 1")
row = conn.execute("SELECT regex_extract('x(ab)', 'xxabaa', 1)").fetchone()
assert row[0] == "ab", row[0]

print("Testing pattern 'x(ab)' WITH capture group = 0")
row = conn.execute("SELECT regex_extract('x(ab)', 'xxabaa', 0)").fetchone()
assert row[0] == "xab", row[0]

print("Testing pattern 'g(oog)+le' WITHOUT capture group")
row = conn.execute("SELECT regex_extract('g(oog)+le', 'googoogoogle')").fetchone()
assert row[0] == "googoogoogle", row[0]

print("Testing pattern 'g(oog)+le' WITH capture group = 1")
row = conn.execute("SELECT regex_extract('g(oog)+le', 'googoogoogle', 1)").fetchone()
assert row[0] == "oog", row[0]

print("Testing pattern '[Cc]at' WITHOUT capture group")
row = conn.execute("SELECT regex_extract('[Cc]at', 'cat')").fetchone()
assert row[0] == "cat", row[0]

print("Testing pattern '[Cc]at' WITHOUT capture group, expecting empty return")
row = conn.execute("SELECT regex_extract('[Cc]at', 'hello')").fetchone()
assert row[0] is None, row[0]

conn.close()


conn2 = sqlite3.connect("test.db", isolation_level=None)
print(f"Testing connection 2: {conn2}")
row = conn2.execute("SELECT regex_extract('x(ab)', 'xxabaa')").fetchone()
assert row[0] == "xab", row[0]


print("All tests passed")
