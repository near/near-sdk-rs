import os
import sys


class Cache:
    def __init__(self, dir):
        self._dir = dir
        print(f"Cache directory: {dir}", file=sys.stderr)
        os.makedirs(self.registry, exist_ok=True)
        os.makedirs(self.git, exist_ok=True)
        os.makedirs(self.target, exist_ok=True)

    @property
    def root(self):
        return self._dir

    @property
    def registry(self):
        return os.path.join(self.root, "registry")

    @property
    def git(self):
        return os.path.join(self.root, "git")

    @property
    def target(self):
        return os.path.join(self.root, "target")
