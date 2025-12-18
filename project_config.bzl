"""
This file defines project-specific metadata used by Bazel macros, such as `dash_license_checker`.
It provides structured configuration that helps determine behavior such as:

- Source language type (used to determine license check file format)
- Safety level or other compliance info (e.g. ASIL level)

"""

PROJECT_CONFIG = {
    "asil_level": "QM",  # or "ASIL-A", "ASIL-B", …
    "source_code": ["rust"],  # Languages used in the module
}
