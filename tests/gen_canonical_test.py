# Auto generate `canonical_cases/mod.rs` from `canonical.yaml`
# Run this when updating `caonical.yaml`

import yaml
import sys
import shutil
import os

try:
    from yaml import CLoader as YamlLoader, CDumper as YamlDumper
except ImportError:
    from yaml import Loader as YamlLoader, Dumper as YamlDumper


def main():
    with open("canonical.yaml", encoding="utf-8") as input_file:
        input_tests = yaml.load(input_file, Loader=YamlLoader)
    print(f"version {input_tests['version']}", file=sys.stderr)
    tests = input_tests["tests"]
    print(f"loaded {len(tests)} tests", file=sys.stderr)

    try:
        shutil.rmtree("canonical_cases")
    except FileNotFoundError:
        pass
    os.mkdir("canonical_cases")

    with open("canonical_cases/mod.rs", "w", encoding="utf-8") as out:
        out.write(TEMPLATE_PRE)
        for name, test in tests.items():
            if name.startswith("test"):
                name = name[4:]
            test_case = yaml.dump(
                test,
                Dumper=YamlDumper,
                allow_unicode=True,
            )
            out.write(f'#[test_case(r#"\n{test_case}"#\n; "{name}")]\n')
        out.write(TEMPLATE_POS)


TEMPLATE_PRE = """//! AUTO GENERATED WITH `gen_canonical_tests.py`
use test_case::test_case;
use super::{runner, TestCase};
"""
TEMPLATE_POS = """fn canonical(input: &str) {
    let test_case: TestCase = serde_yaml::from_str(input).expect("Bad YAML input");
    runner(test_case);
}
"""

if __name__ == "__main__":
    main()
