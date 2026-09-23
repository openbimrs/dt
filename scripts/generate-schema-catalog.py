#!/usr/bin/env python3
"""Derive the ISO 23387 structural catalog from a local copy of the XSD.

First stage of the schema-table pipeline:

    generate-schema-catalog.py <ISO-23387.xsd> <catalog.json>
    generate-schema-tables.py  <catalog.json> [openbim-dt/src/schema.rs]
    cargo fmt --all

The XSD is a legally accessed local copy kept under the ignored `references/`
directory; neither it nor the derived catalog is ever committed. Only
structural facts are recorded (element/attribute names, cardinalities, base
types, patterns, enumerations, source lines for provenance). No
`xs:annotation`/`xs:documentation` prose is read or captured.

Every global element declared by the schema is walked, not only the subset
reachable from one root. Requires `lxml`.
"""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any

from lxml import etree

if len(sys.argv) != 3:
    sys.exit(__doc__.strip())

XSD_PATH = Path(sys.argv[1])
OUTPUT = Path(sys.argv[2])
XSD_NS = "http://www.w3.org/2001/XMLSchema"
X = f"{{{XSD_NS}}}"

DT_NS = "https://standards.iso.org/iso/23387/ed-2/en/"
FILE = "ISO-23387.xsd"


def is_xsd(node: etree._Element, *names: str) -> bool:
    if not isinstance(node.tag, str):
        return False
    qname = etree.QName(node)
    if qname.namespace != XSD_NS:
        return False
    return not names or qname.localname in names


def occurs(value: str | None, default: int = 1) -> int | None:
    if value == "unbounded":
        return None
    return int(value) if value is not None else default


def multiply(left: int | None, right: int | None) -> int | None:
    return None if left is None or right is None else left * right


def local(name: str | None) -> str:
    return (name or "").split(":")[-1]


class SchemaIndex:
    def __init__(self, document: etree._ElementTree) -> None:
        self.complex_types: dict[str, etree._Element] = {}
        self.simple_types: dict[str, etree._Element] = {}
        self.elements: dict[str, etree._Element] = {}
        self.attributes: dict[str, etree._Element] = {}
        root = document.getroot()
        for node in root.findall(f"{X}complexType"):
            if node.get("name"):
                self.complex_types.setdefault(node.get("name"), node)
        for node in root.findall(f"{X}simpleType"):
            if node.get("name"):
                self.simple_types.setdefault(node.get("name"), node)
        for node in root.findall(f"{X}element"):
            if node.get("name"):
                self.elements.setdefault(node.get("name"), node)
        for node in root.findall(f"{X}attribute"):
            if node.get("name"):
                self.attributes.setdefault(node.get("name"), node)


def main() -> None:
    document = etree.parse(str(XSD_PATH))
    index = SchemaIndex(document)

    definitions: dict[str, dict[str, Any]] = {}
    all_element_names: set[str] = set()
    all_attributes: set[str] = set()
    all_enums: set[str] = set()
    choice_counter = 0

    def simple_constraints(type_name: str | None, node: etree._Element) -> tuple[str | None, str | None, list[str]]:
        simple = node.find(f"{X}simpleType")
        if simple is None and type_name:
            simple = index.simple_types.get(local(type_name))
        if simple is None:
            return type_name, None, []
        restriction = simple.find(f"{X}restriction")
        if restriction is None:
            return type_name, None, []
        pattern = restriction.find(f"{X}pattern")
        enums = [n.get("value") for n in restriction.findall(f"{X}enumeration") if n.get("value") is not None]
        return restriction.get("base") or type_name, pattern.get("value") if pattern is not None else None, enums

    def collect_attributes(container: etree._Element, definition: dict[str, Any]) -> None:
        for attribute in container.findall(f"{X}attribute"):
            raw = attribute.get("name") or attribute.get("ref")
            if not raw:
                continue
            name = local(raw)
            declaration = attribute
            if attribute.get("ref"):
                entry = index.attributes.get(name)
                if entry is not None:
                    declaration = entry
            all_attributes.add(name)
            data_type, pattern, enums = simple_constraints(declaration.get("type"), declaration)
            all_enums.update(enums)
            definition["attributes"].append({
                "name": name,
                "required": attribute.get("use") == "required",
                "default": attribute.get("default"),
                "data_type": data_type,
                "pattern": pattern,
                "enum_values": enums,
                "source_line": attribute.sourceline,
            })

    def build_definition(handle: str, name: str, type_node: etree._Element | None,
                          value_node: etree._Element, *, global_element: bool) -> None:
        nonlocal choice_counter
        if handle in definitions:
            return
        all_element_names.add(name)
        definition: dict[str, Any] = {
            "handle": handle,
            "name": name,
            "global": global_element,
            "source_line": value_node.sourceline,
            "attributes": [],
            "children": [],
            "choice_groups": [],
        }
        definitions[handle] = definition

        if type_node is None:
            data_type, pattern, enums = simple_constraints(value_node.get("type"), value_node)
            definition["data_type"] = data_type
            definition["pattern"] = pattern
            definition["enum_values"] = enums
            all_enums.update(enums)
            return

        containers: list[etree._Element] = []
        base_type_name: str | None = None

        def expand(node: etree._Element) -> None:
            nonlocal base_type_name
            content = node.find(f"{X}complexContent")
            if content is not None:
                extension = content.find(f"{X}extension")
                if extension is not None:
                    base_name = local(extension.get("base"))
                    base = index.complex_types.get(base_name)
                    if base is not None:
                        expand(base)
                    base_type_name = base_name if base is not None else base_type_name
                    containers.append(extension)
                    return
                restriction = content.find(f"{X}restriction")
                if restriction is not None:
                    containers.append(restriction)
                    return
            simple_content = node.find(f"{X}simpleContent")
            if simple_content is not None:
                extension = simple_content.find(f"{X}extension")
                if extension is not None:
                    containers.append(extension)
                    return
            containers.append(node)

        expand(type_node)
        if base_type_name:
            definition["base_type"] = base_type_name

        def walk_particle(node: etree._Element, parent_min: int = 1,
                           parent_max: int | None = 1, choice_group: str | None = None) -> None:
            nonlocal choice_counter
            if not isinstance(node.tag, str):
                return
            tag = etree.QName(node).localname
            node_min = occurs(node.get("minOccurs"))
            node_max = occurs(node.get("maxOccurs"))
            effective_min = multiply(parent_min, node_min)
            effective_max = multiply(parent_max, node_max)
            if tag == "choice":
                group = f"{handle}.choice.{choice_counter}"
                choice_counter += 1
                definition["choice_groups"].append({
                    "handle": group,
                    "min_occurs": effective_min,
                    "max_occurs": effective_max,
                    "children": [],
                })
                for child in node:
                    if is_xsd(child):
                        walk_particle(child, 0, effective_max, group)
                return
            if tag in {"sequence", "all"}:
                for child in node:
                    if is_xsd(child):
                        walk_particle(child, effective_min or 0, effective_max, choice_group)
                return
            if tag == "group":
                if node.get("ref"):
                    return
                for child in node:
                    if is_xsd(child):
                        walk_particle(child, effective_min or 0, effective_max, choice_group)
                return
            if tag != "element":
                return

            child_name = local(node.get("ref") or node.get("name"))
            if not child_name:
                return
            all_element_names.add(child_name)

            # `<xs:element ref="X"/>` declares nothing locally: its type and
            # content come from the global `<xs:element name="X">`. Resolve
            # it there, otherwise the ref becomes an empty local definition
            # and every valid child of X is rejected as undeclared. Occurrence
            # constraints still come from the ref particle (`node`) itself.
            declaration = node
            if node.get("ref"):
                global_declaration = index.elements.get(child_name)
                if global_declaration is None:
                    raise SystemExit(f"unresolved element ref {child_name!r} at line {node.sourceline}")
                declaration = global_declaration

            child_type_name = declaration.get("type")
            inline_type = declaration.find(f"{X}complexType")
            child_handle = f"{handle}/{child_name}"
            if inline_type is not None:
                build_definition(child_handle, child_name, inline_type, declaration, global_element=False)
            elif child_type_name and local(child_type_name) in index.complex_types:
                child_handle = local(child_type_name)
                type_element = index.complex_types[child_handle]
                build_definition(child_handle, child_name, type_element, declaration, global_element=False)
            else:
                build_definition(child_handle, child_name, None, declaration, global_element=False)

            if choice_group is not None:
                group = next(c for c in definition["choice_groups"] if c["handle"] == choice_group)
                group["children"].append(child_name)
            definition["children"].append({
                "name": child_name,
                "definition": child_handle,
                "min_occurs": effective_min,
                "max_occurs": effective_max,
                "choice_group": choice_group,
                "source_line": node.sourceline,
            })

        for container in containers:
            collect_attributes(container, definition)
            for particle in container:
                if is_xsd(particle, "sequence", "choice", "all", "group"):
                    walk_particle(particle)

    # Walk every global element declared directly in ISO-23387.xsd — not just
    # the subset reachable from ISO 7817-3's LOIN root.
    root_names = sorted(index.elements.keys())
    for root_name in root_names:
        root_element = index.elements[root_name]
        build_definition(
            root_name, root_name, root_element.find(f"{X}complexType"), root_element,
            global_element=True,
        )

    # Also register every named complex/simple type so scalar contracts (Guid,
    # DateTime-like restrictions, Language, AnyUri, Rational, Scale/Base,
    # ConceptType, etc.) are inventoried even when not directly instantiated
    # by a global root in this pass.
    for type_name, type_node in index.complex_types.items():
        if type_name not in definitions:
            synthetic = etree.Element(f"{X}element")
            synthetic.set("name", type_name)
            synthetic.sourceline = type_node.sourceline
            build_definition(type_name, type_name, type_node, synthetic, global_element=False)
    for type_name, type_node in index.simple_types.items():
        if type_name in definitions:
            continue
        synthetic = etree.Element(f"{X}element")
        synthetic.set("name", type_name)
        synthetic.set("type", type_name)
        synthetic.sourceline = type_node.sourceline
        all_element_names.add(type_name)
        data_type, pattern, enums = simple_constraints(type_name, synthetic)
        all_enums.update(enums)
        definitions[type_name] = {
            "handle": type_name,
            "name": type_name,
            "global": False,
            "source_line": type_node.sourceline,
            "attributes": [],
            "children": [],
            "choice_groups": [],
            "data_type": data_type,
            "pattern": pattern,
            "enum_values": enums,
        }

    for element in document.findall(f".//{X}element"):
        value = element.get("name") or element.get("ref")
        if value:
            all_element_names.add(local(value))
    for attribute in document.findall(f".//{X}attribute"):
        value = attribute.get("name") or attribute.get("ref")
        if value:
            all_attributes.add(local(value))
    for enum in document.findall(f".//{X}enumeration"):
        if enum.get("value"):
            all_enums.add(enum.get("value"))

    catalog = {
        "profile": "ISO 23387:2020 edition 2 (full structural catalog)",
        "namespace": DT_NS,
        "schemas": [
            {"file": FILE, "sha256": hashlib.sha256(XSD_PATH.read_bytes()).hexdigest()},
        ],
        "global_elements": sorted(name for name, d in definitions.items() if d.get("global")),
        "element_names": sorted(all_element_names),
        "attribute_names": sorted(all_attributes),
        "enum_values": sorted(all_enums),
        "elements": definitions,
    }
    OUTPUT.write_text(json.dumps(catalog, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT}: {len(all_element_names)} elements, {len(all_attributes)} attributes, "
        f"{len(all_enums)} enums, {len(definitions)} context definitions, "
        f"{len(catalog['global_elements'])} global roots"
    )


if __name__ == "__main__":
    main()
