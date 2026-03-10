- SIMD operation use the SIMD registers of CPU
- aarch or arm
    - NEON
        - 64 bit register
        - will combine 2 64 bit register to be used as a single 128 bit register
- intel and AMD x86 and x86_64
    - avx2
        - 256 and 128 bit registers
        - common but not the most
    - sse2
        - 128 bit register
        - most commonly supported (steam hardware survey)

### Vectors:
- SIMD values
- Has a fixed value, known at compile time
- All elements are of same type
- Vectors are aligned to it entire size (16 byte, 32 bytes, etc)
- Vector data can be called packed data

### Vectorize
- Operations that use SIMD instructions are referred to as "vectorized"

### Auto-Vectorization
- When the compiler the automatically recognizes that a scalar operation can be improved/replaced
  with the SIMD instructions

### Scalar
- In mathematics, refers to values that can be represented as single elements (individual number e.g.1,2, 4.1)
- Can be used to refer "scalar operations". e.g. Adding a number with another one by one in a for-loop

### Lane
- A single element position in a vector
- reading individual lanes can be costly
- as the on most architectures, the vector has to be pushed out of the SIMD register onto the stack,
  then individual lane is accessed while its on the stack (sometimes pushed to the registers)

### Bit Widths
- The bit-size of the vector involved, not individual elements
- 128-bit SIMD most common: 128 > 64,256 > 512
- 128-bit vectors can be `f32x4`, `i32x4`, `i16x8`, `i8x16`

### Vector Register
- Extra wide registers used for SIMD operations

### Vertical
- SIMD operations are vertical
- each lane is processed individually without regard for the other lanes
- for example a vertical add:
  ```
  vector a |                   | vector b | vector out
  1        | -> lane 0 (+) ->  | 2        | 3
  4        | -> lane 1 (+) ->  | 5        | 9
  2        | -> lane 2 (+) ->  | 3        | 5
  ```
