"""Discover Pydantic BaseModel subclasses in a Python package.

Walks a package for BaseModel subclasses, extracts their JSON Schemas,
and builds per-field source maps mapping JSON Pointers to source locations.
"""

import importlib
import inspect
import re
import sys
from contextlib import contextmanager
from typing import Any, Dict, List, Optional

try:
    from pydantic import BaseModel as _V2BaseModel
except ImportError:
    _V2BaseModel = None

try:
    from pydantic.v1 import BaseModel as _V1BaseModel
except ImportError:
    _V1BaseModel = None


@contextmanager
def _capture_stdout():
    """Redirect stdout to stderr during import to prevent user-code print()
    calls from corrupting the JSON-RPC protocol channel."""
    old_stdout = sys.stdout
    sys.stdout = sys.stderr
    try:
        yield
    finally:
        sys.stdout = old_stdout


def discover_models(package: str) -> Dict[str, Any]:
    """Discover Pydantic BaseModel subclasses in a Python package.

    Returns a dict with:
        models: list of model entries, each with name, module_path, schema, source_map
    """
    with _capture_stdout():
        try:
            mod = importlib.import_module(package)
        except ImportError as e:
            raise ImportError(f"Cannot import package '{package}': {e}") from e

    models = []
    warnings_list = []

    _collect_models(mod, models, warnings_list, package)

    result: Dict[str, Any] = {"models": models}
    if warnings_list:
        result["warnings"] = warnings_list

    return result


def _collect_models(
    mod: Any,
    models: List[Dict[str, Any]],
    warnings_list: List[Dict[str, Any]],
    root_package: str,
    visited: Optional[set] = None,
) -> None:
    if visited is None:
        visited = set()

    mod_id = getattr(mod, "__name__", str(mod))
    if mod_id in visited:
        return
    visited.add(mod_id)

    # Collect BaseModel subclasses defined in this module
    try:
        members = inspect.getmembers(mod, inspect.isclass)
    except Exception:
        members = []

    for name, cls in members:
        if _V2BaseModel is not None and issubclass(cls, _V2BaseModel) and cls is not _V2BaseModel:
            try:
                entry = _extract_model(cls, name)
                models.append(entry)
            except Exception as e:
                warnings_list.append({
                    "type": "extraction_error",
                    "model": name,
                    "message": str(e),
                })
        elif _V1BaseModel is not None and issubclass(cls, _V1BaseModel) and cls is not _V1BaseModel:
            try:
                entry = _extract_model_v1(cls, name)
                models.append(entry)
                warnings_list.append({
                    "type": "pydantic_v1",
                    "model": name,
                    "message": (
                        f"Model '{name}' uses Pydantic v1. "
                        "V1 support is best-effort; upgrade to v2 for full support."
                    ),
                })
            except Exception as e:
                warnings_list.append({
                    "type": "extraction_error",
                    "model": name,
                    "message": str(e),
                })

    # Recurse into submodules
    if hasattr(mod, "__path__"):
        for _, submod_name, _ in pkgutil_iter_modules(mod.__path__, mod.__name__ + "."):
            try:
                submod = importlib.import_module(submod_name)
            except Exception:
                continue
            _collect_models(submod, models, warnings_list, root_package, visited)


def pkgutil_iter_modules(path, prefix):
    """Vendored pkgutil.iter_modules equivalent — avoids importing pkgutil."""
    import pkgutil
    return pkgutil.iter_modules(path=path, prefix=prefix)


def _extract_model(cls, name: str) -> Dict[str, Any]:
    """Extract schema and source map for a Pydantic v2 model."""
    try:
        schema = cls.model_json_schema()
    except Exception as e:
        raise RuntimeError(f"model_json_schema() failed: {e}") from e

    source_map = _build_source_map_v2(cls)
    module_path = cls.__module__

    return {
        "name": name,
        "module_path": module_path,
        "schema": schema,
        "source_map": source_map,
    }


def _extract_model_v1(cls, name: str) -> Dict[str, Any]:
    """Extract schema and source map for a Pydantic v1 model."""
    try:
        schema = cls.schema()
    except Exception as e:
        raise RuntimeError(f"schema() failed: {e}") from e

    # Pydantic v1 uses __fields__ instead of model_fields
    source_map = _build_source_map_v1(cls)
    module_path = cls.__module__

    return {
        "name": name,
        "module_path": module_path,
        "schema": schema,
        "source_map": source_map,
    }


def _build_source_map_v2(cls) -> Dict[str, Any]:
    """Build a source map for Pydantic v2 model fields.

    Returns a dict mapping JSON Pointers (/properties/field_name) to
    {file, line} source locations.
    """
    source_map: Dict[str, Any] = {}

    source_file = inspect.getsourcefile(cls)
    if source_file is None:
        return source_map

    try:
        source_lines, start_line = inspect.getsourcelines(cls)
    except (OSError, TypeError):
        return source_map

    model_fields = getattr(cls, "model_fields", None)
    if model_fields is None:
        return source_map

    for field_name in model_fields:
        decl_line = _find_field_declaration_line(source_lines, start_line, field_name)
        if decl_line is not None:
            pointer = f"/properties/{field_name}"
            source_map[pointer] = {
                "file": source_file,
                "line": decl_line,
            }

    return source_map


def _build_source_map_v1(cls) -> Dict[str, Any]:
    """Build a source map for Pydantic v1 model fields."""
    source_map: Dict[str, Any] = {}

    source_file = inspect.getsourcefile(cls)
    if source_file is None:
        return source_map

    try:
        source_lines, start_line = inspect.getsourcelines(cls)
    except (OSError, TypeError):
        return source_map

    model_fields = getattr(cls, "__fields__", None)
    if model_fields is None:
        return source_map

    for field_name in model_fields:
        decl_line = _find_field_declaration_line(source_lines, start_line, field_name)
        if decl_line is not None:
            pointer = f"/properties/{field_name}"
            source_map[pointer] = {
                "file": source_file,
                "line": decl_line,
            }

    return source_map


def _find_field_declaration_line(
    source_lines: List[str],
    start_line: int,
    field_name: str,
) -> Optional[int]:
    """Find the line number where a field is declared in the source.

    Looks for patterns like:
        field_name: type
        field_name: type = default
        field_name: Annotated[...
    The regex ensures a word boundary after the field name to prevent
    false matches on prefix names (e.g., 'name' matching 'name_prefix').
    """
    pattern = re.compile(rf"^\s*{re.escape(field_name)}\s*:")
    for i, line in enumerate(source_lines):
        if pattern.match(line):
            return start_line + i
    return None
