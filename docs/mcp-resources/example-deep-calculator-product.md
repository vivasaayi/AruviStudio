# Gold-Standard Example Product: Scientific Calculator

## Why This Example Exists

This example shows how a familiar product can still be authored with technical depth. The goal is not to list buttons. The goal is to describe the calculator as a system.

## Product Framing

Name: Scientific Calculator

Intent:

- deterministic evaluation of expressions
- predictable calculator state
- safe expansion from basic arithmetic to scientific and programming modes

## Recommended Semantic Tree

### Area 1: Expression Engine

Purpose:
Turn user input into correct mathematical results while preserving a coherent evaluation model.

Children:

- Capability 1.1: Arithmetic Evaluation
- Capability 1.2: Scientific Evaluation
- Capability 1.3: Validation and Error Surfaces

### Area 2: State and Session Model

Purpose:
Own the calculator's input buffer, memory registers, angle mode, repeat-equals behavior, and history state.

Children:

- Capability 2.1: Input State Machine
- Capability 2.2: Memory Registers
- Capability 2.3: History and Replay

### Area 3: Interaction and UX

Purpose:
Render keypad, expression display, result display, keyboard shortcuts, and responsive interaction behavior without violating the engine model.

Children:

- Capability 3.1: Keypad Interaction
- Capability 3.2: Expression Display
- Capability 3.3: Accessibility and Keyboard Control

## Example Of Good Technical Depth

### Capability 1.1: Arithmetic Evaluation

Responsibility:
Evaluate arithmetic expressions correctly across chained inputs and grouping.

Boundary:
Owns parsing and evaluation rules. Does not own visual keyboard layout.

Inputs:

- digit and operator input
- grouped expressions
- repeated equals

Outputs:

- result value
- normalized expression state
- validation or parse errors when input is invalid

Invariants:

- operator precedence is deterministic
- grouping is respected
- repeated equals behaves consistently
- invalid expressions do not corrupt calculator state

Failure modes:

- unmatched parentheses
- incomplete trailing operators
- divide by zero
- invalid chained operations after error state

Rollouts:

- Rollout 1.1.1: Basic Operations
- Rollout 1.1.2: Parentheses and Grouping
- Rollout 1.1.3: Repeated Equals Semantics

### Feature Set 1.4: Powers and Roots Book

Purpose:
Describe the operator family in a readable, book-grade way before deriving delivery work.

Supported structure:

- Capability 1.4.1: Square Operator
- Capability 1.4.2: Square Root Operator
- Capability 1.4.3: Power-of-Y Operator

For each operator chapter, use children like:

- `Reference`: What It Is
- `Reference`: Worked Examples
- `Rollout`: Implementation
- `Rollout`: Tests and Validation

This is the correct Aruvi shape because the operator chapter owns the meaning, while the rollouts remain execution leaves.

Incorrect shape:

- `Rollout`: Powers and Roots
  - `Reference`: What It Is
  - `Reference`: Examples
  - `Rollout`: Implementation

That fails because rollout nodes cannot own deeper structural children.

### Capability 2.1: Input State Machine

Responsibility:
Control how user input mutates current expression, current result, and pending operator state.

Depth cues:

- distinguishes entry state from result state
- defines when a new digit starts a fresh expression versus extends the current one
- explains how delete, clear, and clear-entry differ
- explains post-error recovery

## Delivery Derivation

From the documented structure, work can be derived cleanly:

- `Implement arithmetic parser and evaluator` under `Arithmetic Evaluation`
- `Add grouped-expression tests` under `Parentheses and Grouping`
- `Support repeated equals state transitions` under `Repeated Equals Semantics`

## Why This Is Better Than A Shallow Tree

Shallow version:

- Basic Calculator
- Scientific Functions
- History

Deep Aruvi version:

- explains responsibility
- reveals boundaries and state
- makes rollout staging obvious
- gives validation targets for engineering and QA
