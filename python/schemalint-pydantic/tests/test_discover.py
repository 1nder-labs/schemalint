"""Tests for the discover module."""

import sys
import os
import importlib
import tempfile
import textwrap

import pytest

# Ensure the package is importable
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from schemalint_pydantic.discover import discover_models, _find_field_declaration_line


@pytest.fixture
def temp_package():
    """Create a temporary Python package with Pydantic v2 models."""
    import uuid
    pkg_name = f"testmodels_{uuid.uuid4().hex[:8]}"
    with tempfile.TemporaryDirectory() as tmpdir:
        pkg_dir = os.path.join(tmpdir, pkg_name)
        os.makedirs(pkg_dir)
        init_path = os.path.join(pkg_dir, "__init__.py")

        init_content = textwrap.dedent("""
        from pydantic import BaseModel, Field
        from typing import Optional, Annotated

        class SimpleModel(BaseModel):
            name: str
            age: int

        class NestedModel(BaseModel):
            title: str
            child: SimpleModel

        class EmptyModel(BaseModel):
            pass

        class ModelWithDefault(BaseModel):
            status: str = "active"
            count: int = 0
        """)

        with open(init_path, "w") as f:
            f.write(init_content)

        # Add parent to sys.path
        sys.path.insert(0, tmpdir)
        try:
            yield pkg_name
        finally:
            sys.path.remove(tmpdir)
            # Clean up cached module to prevent cross-test contamination
            sys.modules.pop(pkg_name, None)
            importlib.invalidate_caches()


class TestFindFieldDeclarationLine:
    """Tests for _find_field_declaration_line."""

    def test_simple_declaration(self):
        lines = ["class Foo(BaseModel):\n", "    name: str\n", "    age: int\n"]
        result = _find_field_declaration_line(lines, 10, "name")
        assert result == 11

    def test_declaration_with_default(self):
        lines = ["class Foo(BaseModel):\n", "    status: str = 'active'\n"]
        result = _find_field_declaration_line(lines, 5, "status")
        assert result == 6

    def test_field_not_found(self):
        lines = ["class Foo(BaseModel):\n", "    name: str\n"]
        result = _find_field_declaration_line(lines, 1, "nonexistent")
        assert result is None

    def test_empty_lines(self):
        lines = []
        result = _find_field_declaration_line(lines, 1, "field")
        assert result is None


class TestDiscoverModels:
    """Integration tests for discover_models."""

    def test_discovers_simple_model(self, temp_package):
        try:
            result = discover_models(temp_package)
        except ImportError:
            pytest.skip("pydantic not installed")

        models = result["models"]
        names = {m["name"] for m in models}
        assert "SimpleModel" in names
        assert "NestedModel" in names
        assert "EmptyModel" in names
        assert "ModelWithDefault" in names

    def test_simple_model_has_schema(self, temp_package):
        try:
            result = discover_models(temp_package)
        except ImportError:
            pytest.skip("pydantic not installed")

        simple = next(m for m in result["models"] if m["name"] == "SimpleModel")
        assert "schema" in simple
        schema = simple["schema"]
        assert "properties" in schema
        assert "name" in schema["properties"]
        assert "age" in schema["properties"]

    def test_simple_model_has_source_map(self, temp_package):
        try:
            result = discover_models(temp_package)
        except ImportError:
            pytest.skip("pydantic not installed")

        simple = next(m for m in result["models"] if m["name"] == "SimpleModel")
        source_map = simple["source_map"]
        assert "/properties/name" in source_map
        assert "/properties/age" in source_map
        assert "file" in source_map["/properties/name"]
        assert "line" in source_map["/properties/name"]

    def test_empty_model_has_valid_schema(self, temp_package):
        try:
            result = discover_models(temp_package)
        except ImportError:
            pytest.skip("pydantic not installed")

        empty = next(m for m in result["models"] if m["name"] == "EmptyModel")
        schema = empty["schema"]
        assert "properties" in schema
        assert schema["properties"] == {}
        assert empty["source_map"] == {}

    def test_nested_model_has_child_reference(self, temp_package):
        try:
            result = discover_models(temp_package)
        except ImportError:
            pytest.skip("pydantic not installed")

        nested = next(m for m in result["models"] if m["name"] == "NestedModel")
        schema = nested["schema"]
        assert "child" in schema["properties"]

    def test_invalid_package_raises(self):
        with pytest.raises(ImportError, match="Cannot import"):
            discover_models("nonexistent.package.xyz")


class TestDiscoverErrors:
    """Error path tests."""

    def test_non_package_string_raises(self):
        # A builtin module with no path won't have BaseModel subclasses
        result = discover_models("json")
        assert result["models"] == []
