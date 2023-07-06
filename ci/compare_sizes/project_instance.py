import os
import glob
import subprocess
import tempfile
import shutil
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

                # copy build artifacts, if any
                try:
                    shutil.copytree(self.docker_target_dir(), branch_project.docker_target_dir())
                except Exception:
                    pass

                yield branch_project
        finally:
            repo.git.worktree("prune")

    def examples_dir(self):
        return os.path.join(self.root_dir, "examples")

    def docker_target_dir(self):
        return os.path.join(self.root_dir, "docker-target")

    def build_artifacts(self, *build_args):
        return subprocess.run([os.path.join(self.examples_dir(), "build_all_docker.sh"), *build_args], stdout = subprocess.DEVNULL)

    def sizes(self, *build_args):
        self.build_artifacts(*build_args)

        artifact_paths = glob.glob(self.examples_dir() + '/*/res/*.wasm')
        return {os.path.basename(path):os.stat(path).st_size for path in artifact_paths}
