# PLAN — owned-type XML codec (ROADMAP milestone 3)

Branch `feature/owned-codec`. Consumer: `openbim-loin` needs to read and write
embedded ISO 23387 content (ObjectType, Property, Unit, ...) through the typed
model. Re-read this after context compression.

## Goal

Every owned type round-trips losslessly through `dt::Element` and emits
schema-valid content, with the child-name set checked against the generated
`schema.rs` tables so a regenerated schema that adds content fails a test.

## Evidence the schema tables give (schema.rs, generated)

ConceptType: attrs `about` (opt anyURI), `GUID` (req), `dateOfCreation` (req).
Children, all 0..* in ONE repeating choice (order-free, min 1 total):
Name, Definition, Description, Example (MultiLanguageText); ReferenceDocumentRef,
SimilarToRef, ReplacedObjectsRef, DictionaryRef (Reference); LanguageOfCreator
(xs:language); CountryOfOrigin, Status, DeprecationExplanation (xs:string);
VisualRepresentation (xs:base64Binary); MajorVersion, MinorVersion
(xs:nonNegativeInteger).

Every concrete type extends ConceptType with extra groups/sequence children.
Validator (conformance.rs) allows interleaving only WITHIN one choice group;
across groups and sequence children, declared order is enforced. So the
writer emits in declared `children` order and the reader accepts any order
within a group.

## Owned model gaps (verified)

| Type | Field | Schema | Owned today |
| --- | --- | --- | --- |
| Concept | definition | 0..* | exactly 1 |
| Concept | about + 10 child kinds | declared | absent |
| Concept | names | 0..* (choice min 1) | >=1 via `new` |
| Subject | IsSubtypeOfRef | 0..* | Option |
| Property | DimensionRef | 0..* | Option |
| DataTemplate | HasObjectTypeRef | 0..* | Option |
| Concept.references | — | which child? | ambiguous (see below) |

`Concept.references` has no single wire element: ConceptType declares four
Reference-typed children. RESOLVED: nothing in dt parses into owned
`Concept` today (no `to_owned` for concepts; only MultilingualTextRef and
ReferenceRef convert), so `references` has no wire meaning yet. Decision:
map it to `ReferenceDocumentRef` (its only documentation-shaped sense) and
add explicit `similar_to_refs`, `replaced_objects_refs`, `dictionary_refs`.
Document the mapping on the getter.

## Whole-schema findings (from schema.rs, 2026-09-23)

- Sequence children, not choice: Dimension 7 exponents (1..1 each),
  QuantityKind DimensionRef (1..1), Unit Symbol*, DimensionRef, Scale, Base,
  Coefficient, Offset (all 1..1 but Symbol), ReferenceDocument
  DateOfPublication?, Author?, ISBN?, Language+, Publisher?, URI?,
  Property DataType (1..1) + IsSpecializationOfRef (0..1). Writer MUST emit
  these in declared order.
- Choice groups with min 1: SubjectType group 1 = {HasPartRef, IsSubtypeOfRef}
  requires at least ONE of them. So an ObjectType with neither is
  schema-INVALID. `Subject::new(concept)` today builds exactly that. Same for
  GroupOfProperties group 2 {HasPropertyRef} (owned already requires one) and
  DataTemplate group 2 {HasObjectTypeRef, HasPropertyRef,
  HasGroupOfPropertiesRef}. DataType choice {Min/Max*, DataFormat,
  PossibleValues} also min 1.
- Decision: owned types may hold schema-invalid-but-well-typed states only
  where construction-time enforcement would break the existing constructors.
  `to_element` never fails; instead provide `validate_schema()` on the owned
  value path via Document::standalone and document that `Subject::new` needs
  a has-part or subtype ref to be schema-valid. Rationale: refusing at
  construction breaks every existing caller; loin's authoring layer then
  surfaces the violation through validate().
  REVISIT only if the user wants construction-time enforcement.
- DataType constraints `value` attr is xs:anySimpleType (owned: String). OK.
- Rational pattern `[+-]?[0-9]+(/[1-9][0-9]*)?` matches `Rational::from_str`.
- ReferenceDocument DateOfPublication is xs:dateTime (owned DateTime ok).
- New scalar kinds: VisualRepresentation base64Binary, Major/MinorVersion
  nonNegativeInteger. Follow the crate's existing convention (validated,
  lexeme-preserving newtypes like `PositiveInteger`, `Rational`): add
  `NonNegativeInteger(String)` and `Base64Binary(String)` in value.rs with
  FromStr + ValueErrorKind variants. Lexeme preservation keeps round trips
  exact (`007` stays `007`), and unbounded values stay representable.

## Decisions

1. Widen, don't remove. New plural storage; old singular getters return the
   first element and are `#[deprecated]` with a pointer to the plural.
   `Concept::new` keeps its signature (name + definition) so no consumer
   breaks at construction. Setter `set_definition` replaces all definitions
   with one — documented.
2. Version: additive API + deprecations, but `Concept` gains fields and its
   `PartialEq` changes meaning only for new data, and `Option` setters keep
   working. Target **0.3.0** anyway: changed `Property::set_dimension_ref`
   semantics etc. are observable. Decide finally from `cargo semver-checks`
   if available, else the API diff.
3. Codec entry points: `to_element(&self) -> Element` (dt-namespaced, prefix
   `dt`, no xmlns) and `from_element(&Element) -> Result<Self, CodecError>`
   which reads content only (ignores outer name/namespace; LOIN embeds under
   an unqualified outer element).
4. `CodecError { kind, path }` with stable `CodecErrorKind`: MissingAttribute,
   InvalidValue, MissingChild, TooManyChildren, UnknownChild. Unknown
   children are refused (owned values can't retain them; the wire tree does).
5. Scalar children typed where dt has a type (Language, Decimal, DateTime,
   AnyUri); new newtypes only if needed for base64/nonNegativeInteger
   validation — otherwise `String` validated at decode.

## Evidence plan

a. maximal round trip per type (every child kind, 2 instances where 0..*)
b. maximal output has zero `validate_schema` violations (non-root kinds via a
   test-only entry point against their Definition)
c. drift guard: codec's handled child names == DEFINITIONS children per handle
d. idempotence over `tests/fixtures/synthetic-library.xml`
e. mutation probes in the dt mutation harness
f. `./scripts/gate.sh` exit 0

## Status

- [x] public Element/Attribute builder + Document::standalone (d159cdb)
- [ ] read current parse->owned code; resolve Concept.references mapping
- [ ] widen owned contracts
- [ ] codec + evidence
- [ ] docs, CHANGELOG, gate, review, push, release
