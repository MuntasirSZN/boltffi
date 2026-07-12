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
public class BoltffiJavaMutationBench {
    @Setup(Level.Trial)
    public void verifyMutationBehavior() {
        long[] values = {41L};
        BenchBoltFFI.incU64(values);
        if (values[0] != 42L) throw new AssertionError("inc_u64 behavior mismatch");
    }

    @Benchmark
    public void boltffi_java_inc_u64(Blackhole blackhole) {
        long[] values = {0L};
        BenchBoltFFI.incU64(values);
        blackhole.consume(values[0]);
    }
}
