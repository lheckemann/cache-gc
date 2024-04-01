#!/usr/bin/env python3

from glob import glob
import json
import os
import re
import sys


NARINFO_PREFIX = 'StorePath: '


def get_store_path(hash):
    with open(f"{hash}.narinfo") as f:
        first_line = f.readline().strip()
        assert first_line.startswith(NARINFO_PREFIX)
        return first_line[len(NARINFO_PREFIX):]


def enrich(key, path_info):
    hash = re.search('/?(?P<hash>[0-9a-z]{32})-?', key).group('hash')
    path_info['registrationTime'] = round(os.stat(f"{hash}.narinfo").st_mtime)
    path_info['path'] = get_store_path(hash)

    return path_info


print(json.dumps(list(
    enrich(k, v) for k, v in json.load(sys.stdin).items()
)))
