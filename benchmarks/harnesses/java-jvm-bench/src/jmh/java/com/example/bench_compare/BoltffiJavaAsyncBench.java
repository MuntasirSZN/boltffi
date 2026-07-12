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
public class BoltffiJavaAsyncBench {
    @Setup(Level.Trial)
    public void verifyAsyncBehavior() {
        if (BenchBoltFFI.asyncAdd(100, 200).join() != 300) {
            throw new AssertionError("async_add behavior mismatch");
        }
    }

    @Benchmark
    public void boltffi_java_async_add(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.asyncAdd(100, 200).join());
    }
}
