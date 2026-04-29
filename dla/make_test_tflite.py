"""make_test_tflite.py — produce the smallest possible INT8 TFLite model
that exercises both Conv2D and a non-trivial activation, suitable for
a single-shot APU 650 round-trip via ncc-tflite.

Outputs:
    test_int8.tflite    — INT8-quantised, 1×8×8×1 input, 1×8×8×4 output
    test_input.bin      — random INT8 input tensor for verification
    test_output_cpu.bin — CPU reference output for bit-comparison after APU run

Usage:
    pip install --user 'tensorflow==2.15' numpy
    python make_test_tflite.py
"""
import numpy as np
import tensorflow as tf

OUT_DIR = "."
INPUT_SHAPE = (1, 8, 8, 1)
OUT_CHANNELS = 4

def build_model() -> tf.keras.Model:
    inp = tf.keras.Input(shape=INPUT_SHAPE[1:], dtype=tf.float32, name="x")
    x = tf.keras.layers.Conv2D(
        OUT_CHANNELS, 3, padding="same",
        kernel_initializer=tf.keras.initializers.Constant(0.1),
        bias_initializer="zeros",
        name="conv0",
    )(inp)
    out = tf.keras.layers.ReLU(name="relu0")(x)
    return tf.keras.Model(inp, out, name="apu_smoke")

def representative_dataset():
    rng = np.random.default_rng(0)
    for _ in range(32):
        yield [rng.standard_normal(INPUT_SHAPE).astype(np.float32) * 0.5]

def main() -> None:
    model = build_model()
    converter = tf.lite.TFLiteConverter.from_keras_model(model)
    converter.optimizations = [tf.lite.Optimize.DEFAULT]
    converter.representative_dataset = representative_dataset
    converter.target_spec.supported_ops = [tf.lite.OpsSet.TFLITE_BUILTINS_INT8]
    converter.inference_input_type = tf.int8
    converter.inference_output_type = tf.int8
    tflite_bytes = converter.convert()

    with open(f"{OUT_DIR}/test_int8.tflite", "wb") as f:
        f.write(tflite_bytes)
    print(f"wrote test_int8.tflite ({len(tflite_bytes)} bytes)")

    interp = tf.lite.Interpreter(model_content=tflite_bytes)
    interp.allocate_tensors()
    in_d = interp.get_input_details()[0]
    out_d = interp.get_output_details()[0]
    print(f"input  shape={in_d['shape']} dtype={in_d['dtype']} scale={in_d['quantization_parameters']}")
    print(f"output shape={out_d['shape']} dtype={out_d['dtype']} scale={out_d['quantization_parameters']}")

    rng = np.random.default_rng(42)
    sample = rng.integers(-32, 32, size=INPUT_SHAPE, dtype=np.int8)
    sample.tofile(f"{OUT_DIR}/test_input.bin")
    interp.set_tensor(in_d["index"], sample)
    interp.invoke()
    out = interp.get_tensor(out_d["index"])
    out.tofile(f"{OUT_DIR}/test_output_cpu.bin")
    print(f"wrote test_input.bin ({sample.nbytes} bytes)")
    print(f"wrote test_output_cpu.bin ({out.nbytes} bytes)")
    print("\nNow:  ./compile_dla.sh test_int8.tflite mdla3.0")

if __name__ == "__main__":
    main()
