from ntpath import join
import subprocess
import json
import tempfile
import os
import shutil
import tarfile

class cargo_package:
    temp_dir = tempfile.TemporaryDirectory()
    dependencies = []

    def __init__(self, cargo_package):
        self.metadata = json.loads(subprocess.check_output(
            f"cargo metadata --format-version 1 --no-deps --manifest-path {cargo_package}/Cargo.toml", shell=True))
        cargo_crate = self.metadata["packages"][0]
        self.name = cargo_crate["name"]
        self.version = cargo_crate["version"]
        self.maintainer = ",".join(cargo_crate["authors"])
        self.description = cargo_crate["description"]
        self.license = cargo_crate["license"]
        self.homepage = cargo_crate["homepage"]
        self.tags = cargo_crate["keywords"]
        self.architecture = "aarch64-3.10"
        self.priority = "Optional"

    def install(self, file, install_dir):
        out_dir = f"{self.temp_dir.name}/DATA/{install_dir}/"
        if not os.path.exists(out_dir):
            os.makedirs(out_dir, exist_ok=True)
        shutil.copy(file, out_dir)
    
    def install_dir(self, dir, install_dir):
        out_dir = f"{self.temp_dir.name}/DATA/{install_dir}"
        shutil.copytree(dir, out_dir)

    def add_dependency(self, dependency):
        self.dependencies.append(dependency)

    def __control_dir(self):
        control_dir = f"{self.temp_dir.name}/CONTROL"
        if not os.path.exists(control_dir):
            os.makedirs(control_dir, exist_ok=True)
        return control_dir

    def postinst(self, script):
        with open(f"{self.__control_dir()}/postinst", "w") as postinst:
            postinst.write(script)

    def prerm(self, script):
        with open(f"{self.__control_dir()}/prerm", "w") as prerm:
            prerm.write(script)

    def __write_control(self):
        with open(f"{self.__control_dir()}/control", "w") as f:
            f.write(f"Package: {self.name}\n")
            f.write(f"Architecture: {self.architecture}\n")
            f.write(f"Maintainer: {self.maintainer}\n")
            f.write(f"Description: {self.description}\n")
            f.write(f"Priority: {self.priority}\n")
            if self.dependencies:
                f.write(f"Depends:{','.join(self.dependencies)}\n")
            if self.license:
                f.write(f"Priority: {self.license}\n")
            if self.homepage:
                f.write(f"Homepage: {self.homepage}\n")
            if self.tags:
                f.write(f"Tags: {self.tags}\n")

            # install_size = subprocess.check_output(
            #     "du -s " + self.temp_dir.name + "/DATA | awk '\{print $1; exit\}'", shell=True)
            # f.write(f"Installed-Size: {install_size}\n")

    def __copy_binary(self):
        bin_name = self.metadata["packages"][0]["targets"][0]["name"]
        file = f'{self.metadata["target_directory"]}/{os.environ["RUST_TARGET"]}/release/{bin_name}'
        self.install(file, f"/opt/sbin")

    def create_package(self, output_dir, group, owner):
        if not os.path.exists(output_dir):
            os.mkdir(output_dir)

        self.__copy_binary()
        self.__write_control()

        package_name = f"{self.name}_{self.version}.{self.architecture}.ipk"
        with open(f"{self.temp_dir.name}/debian-binary", "w") as f:
            f.write("2.0")

        current_dir = os.getcwd()
        os.chdir(self.__control_dir())
        with tarfile.open("../control.tar.gz", "w:gz") as ctrl:
            ctrl.add(".")
        
        os.chdir(f"{self.temp_dir.name}/DATA/")
        with tarfile.open("../data.tar.gz", "w:gz") as data:
            data.add(".")

        os.chdir(self.temp_dir.name)
        with tarfile.open(f"{current_dir}/{output_dir}/{package_name}", "w:gz") as targz:
            targz.add("./debian-binary")
            targz.add("./data.tar.gz")
            targz.add("./control.tar.gz")

        os.chdir(current_dir)

def create_repository(packages, output_dir):
    for pkg in packages:
        pkg.create_package(output_dir, "root", "admin")
    subprocess.call(
        f"opkg-make-index {output_dir}/. > {output_dir}/Packages", shell=True)