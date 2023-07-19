import docker
import os
import glob
import subprocess
import tempfile
import shutil
import sys
from contextlib import contextmanager
from git import Repo


class ProjectInstance:
    def __init__(self, root_dir):
        self.root_dir = root_dir

    @contextmanager
    def branch(self, branch):
        repo = Repo(self.root_dir)

        try:
            with tempfile.TemporaryDirectory() as tempdir:
                repo.git.worktree("add", tempdir, branch)
                branch_project = ProjectInstance(tempdir)

                yield branch_project
        finally:
            repo.git.worktree("prune")

    def examples_dir(self):
        return os.path.join(self.root_dir, "examples")

    def docker_target_dir(self):
        return os.path.join(self.root_dir, "docker-target")

    def _build_artifact(self, artifact, cache):
        client = docker.from_env()

        client.containers.run(
            "nearprotocol/contract-builder:latest-arm64",
            "./build.sh",
            mounts=[
                docker.types.Mount("/host", self.root_dir, type="bind"),
                docker.types.Mount(
                    "/usr/local/cargo/registry", cache.registry, type="bind"
                ),
                docker.types.Mount("/usr/local/cargo/git", cache.git, type="bind"),
                docker.types.Mount("/target", cache.target, type="bind"),
            ],
            working_dir=f"/host/examples/{artifact.name}",
            cap_add=["SYS_PTRACE"],
            security_opt=["seccomp=unconfined"],
            auto_remove=True,
            user=os.getuid(),
            environment={
                "RUSTFLAGS": "-C link-arg=-s",
                "CARGO_TARGET_DIR": "/target",
            },
        )

    def _examples(self):
        return filter(os.DirEntry.is_dir, os.scandir(self.examples_dir()))

    def build_artifacts(self, cache):
        for example in self._examples():
            print(f"Building {example.name}...", file=sys.stderr)
            self._build_artifact(example, cache)

    def sizes(self, cache):
        self.build_artifacts(cache)

        artifact_paths = glob.glob(self.examples_dir() + "/*/res/*.wasm")
        return {
            os.path.basename(path): os.stat(path).st_size for path in artifact_paths
        }
