# F5 rule-source byte fixtures

These exact rule-body images are the finite source set used by the selected
`MeasuredIrulesLoaderProfile` in `rust/tcl-registry/src/irules_policy.rs`.
The profile compares complete input bytes, rejects NUL, and declines unsupported
non-ASCII inputs. Membership is purpose-specific; these fixtures do not define a
general Unicode identifier grammar, normalization rule, key identity, native
parser behavior, object representation, or compiler registration.
