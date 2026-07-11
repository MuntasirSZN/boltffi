package com.example.bench_compare;

import com.example.bench_boltffi.BenchBoltFFI;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.Benchmark;
import org.openjdk.jmh.annotations.BenchmarkMode;
import org.openjdk.jmh.annotations.Level;
import org.openjdk.jmh.annotations.Mode;
import org.openjdk.jmh.annotations.OutputTimeUnit;
import org.openjdk.jmh.annotations.Scope;
import org.openjdk.jmh.annotations.Setup;
import org.openjdk.jmh.annotations.State;
import org.openjdk.jmh.infra.Blackhole;

@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.NANOSECONDS)
@State(Scope.Thread)
public class BoltffiJavaPrimitiveBench {
    @Setup(Level.Trial)
    public void verifyPrimitiveBehavior() {
        if (!BenchBoltFFI.echoBool(true) || BenchBoltFFI.echoBool(false)) {
            throw new AssertionError("echo_bool behavior mismatch");
        }
        if (BenchBoltFFI.negateBool(true) || !BenchBoltFFI.negateBool(false)) {
            throw new AssertionError("negate_bool behavior mismatch");
        }
        if (BenchBoltFFI.echoI8((byte) -101) != (byte) -101) {
            throw new AssertionError("echo_i8 behavior mismatch");
        }
        if (BenchBoltFFI.echoU8((byte) -1) != (byte) -1) {
            throw new AssertionError("echo_u8 behavior mismatch");
        }
        if (BenchBoltFFI.echoI16((short) -30_001) != (short) -30_001) {
            throw new AssertionError("echo_i16 behavior mismatch");
        }
        if (BenchBoltFFI.echoU16((short) -1) != (short) -1) {
            throw new AssertionError("echo_u16 behavior mismatch");
        }
        if (BenchBoltFFI.echoI32(Integer.MIN_VALUE) != Integer.MIN_VALUE) {
            throw new AssertionError("echo_i32 behavior mismatch");
        }
        if (BenchBoltFFI.addI32(100, 200) != 300) {
            throw new AssertionError("add_i32 behavior mismatch");
        }
        if (BenchBoltFFI.echoU32(-1) != -1) {
            throw new AssertionError("echo_u32 behavior mismatch");
        }
        if (BenchBoltFFI.echoI64(Long.MIN_VALUE) != Long.MIN_VALUE) {
            throw new AssertionError("echo_i64 behavior mismatch");
        }
        if (BenchBoltFFI.echoU64(-1L) != -1L) {
            throw new AssertionError("echo_u64 behavior mismatch");
        }
        if (Float.floatToRawIntBits(BenchBoltFFI.echoF32(-0.0F))
                != Float.floatToRawIntBits(-0.0F)) {
            throw new AssertionError("echo_f32 behavior mismatch");
        }
        if (Float.floatToRawIntBits(BenchBoltFFI.addF32(1.25F, 2.5F))
                != Float.floatToRawIntBits(3.75F)) {
            throw new AssertionError("add_f32 behavior mismatch");
        }
        if (Double.doubleToRawLongBits(BenchBoltFFI.echoF64(-0.0))
                != Double.doubleToRawLongBits(-0.0)) {
            throw new AssertionError("echo_f64 behavior mismatch");
        }
        if (Double.doubleToRawLongBits(BenchBoltFFI.addF64(1.25, 2.5))
                != Double.doubleToRawLongBits(3.75)) {
            throw new AssertionError("add_f64 behavior mismatch");
        }
        if (BenchBoltFFI.echoUsize(-1L) != -1L) {
            throw new AssertionError("echo_usize behavior mismatch");
        }
        if (BenchBoltFFI.echoIsize(-42L) != -42L) {
            throw new AssertionError("echo_isize behavior mismatch");
        }
        BenchBoltFFI.noop();
        if (BenchBoltFFI.add(100, 200) != 300) {
            throw new AssertionError("add behavior mismatch");
        }
        if (Double.doubleToRawLongBits(BenchBoltFFI.multiply(2.5, 4.0))
                != Double.doubleToRawLongBits(10.0)) {
            throw new AssertionError("multiply behavior mismatch");
        }
        if (BenchBoltFFI.incU64Value(41L) != 42L) {
            throw new AssertionError("inc_u64_value behavior mismatch");
        }
    }

    @Benchmark
    public void boltffi_java_echo_bool(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoBool(true));
    }

    @Benchmark
    public void boltffi_java_negate_bool(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.negateBool(true));
    }

    @Benchmark
    public void boltffi_java_echo_i8(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoI8((byte) -101));
    }

    @Benchmark
    public void boltffi_java_echo_u8(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoU8((byte) -1));
    }

    @Benchmark
    public void boltffi_java_echo_i16(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoI16((short) -30_001));
    }

    @Benchmark
    public void boltffi_java_echo_u16(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoU16((short) -1));
    }

    @Benchmark
    public void boltffi_java_echo_i32(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoI32(Integer.MIN_VALUE));
    }

    @Benchmark
    public void boltffi_java_add_i32(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.addI32(100, 200));
    }

    @Benchmark
    public void boltffi_java_echo_u32(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoU32(-1));
    }

    @Benchmark
    public void boltffi_java_echo_i64(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoI64(Long.MIN_VALUE));
    }

    @Benchmark
    public void boltffi_java_echo_u64(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoU64(-1L));
    }

    @Benchmark
    public void boltffi_java_echo_f32(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoF32(-0.0F));
    }

    @Benchmark
    public void boltffi_java_add_f32(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.addF32(1.25F, 2.5F));
    }

    @Benchmark
    public void boltffi_java_echo_f64(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoF64(-0.0));
    }

    @Benchmark
    public void boltffi_java_add_f64(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.addF64(1.25, 2.5));
    }

    @Benchmark
    public void boltffi_java_echo_usize(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoUsize(-1L));
    }

    @Benchmark
    public void boltffi_java_echo_isize(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoIsize(-42L));
    }

    @Benchmark
    public void boltffi_java_noop(Blackhole blackhole) {
        BenchBoltFFI.noop();
        blackhole.consume(0);
    }

    @Benchmark
    public void boltffi_java_add(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.add(100, 200));
    }

    @Benchmark
    public void boltffi_java_multiply(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.multiply(2.5, 4.0));
    }

    @Benchmark
    public void boltffi_java_inc_u64_value(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.incU64Value(41L));
    }
}
