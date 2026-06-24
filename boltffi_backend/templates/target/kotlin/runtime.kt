private object Utf8Codec {
    fun maxBytes(value: String): Int = value.length * 3
}

private class WireReader(private val bytes: ByteArray) {
    private var position = 0

    fun readBool(): Boolean = readI8() != 0.toByte()

    fun readI8(): Byte {
        val value = bytes[position]
        position += 1
        return value
    }

    fun readU8(): UByte = readI8().toUByte()

    fun readI16(): Short {
        val value =
            (bytes[position].toInt() and 0xff) or
                ((bytes[position + 1].toInt() and 0xff) shl 8)
        position += 2
        return value.toShort()
    }

    fun readU16(): UShort = readI16().toUShort()

    fun readI32(): Int {
        val value =
            (bytes[position].toInt() and 0xff) or
                ((bytes[position + 1].toInt() and 0xff) shl 8) or
                ((bytes[position + 2].toInt() and 0xff) shl 16) or
                ((bytes[position + 3].toInt() and 0xff) shl 24)
        position += 4
        return value
    }

    fun readU32(): UInt = readI32().toUInt()

    fun readI64(): Long {
        val low = readI32().toLong() and 0xffffffffL
        val high = readI32().toLong() and 0xffffffffL
        return low or (high shl 32)
    }

    fun readU64(): ULong = readI64().toULong()

    fun readF32(): Float = java.lang.Float.intBitsToFloat(readI32())

    fun readF64(): Double = java.lang.Double.longBitsToDouble(readI64())

    fun readOptionalBool(): Boolean? = readOptional { it.readBool() }

    fun readOptionalI8(): Byte? = readOptional { it.readI8() }

    fun readOptionalU8(): UByte? = readOptional { it.readU8() }

    fun readOptionalI16(): Short? = readOptional { it.readI16() }

    fun readOptionalU16(): UShort? = readOptional { it.readU16() }

    fun readOptionalI32(): Int? = readOptional { it.readI32() }

    fun readOptionalU32(): UInt? = readOptional { it.readU32() }

    fun readOptionalI64(): Long? = readOptional { it.readI64() }

    fun readOptionalU64(): ULong? = readOptional { it.readU64() }

    fun readOptionalF32(): Float? = readOptional { it.readF32() }

    fun readOptionalF64(): Double? = readOptional { it.readF64() }

    fun readString(): String {
        val length = readU32().toInt()
        val value = String(bytes, position, length, Charsets.UTF_8)
        position += length
        return value
    }

    fun readBytes(): ByteArray {
        val length = readU32().toInt()
        val value = bytes.copyOfRange(position, position + length)
        position += length
        return value
    }

    private inline fun <T> readOptional(read: (WireReader) -> T): T? {
        return when (readU8()) {
            0.toUByte() -> null
            1.toUByte() -> read(this)
            else -> throw IllegalArgumentException("invalid optional wire tag")
        }
    }
}

private class WireWriter(initialCapacity: Int) {
    private var buffer = java.nio.ByteBuffer
        .allocateDirect(initialCapacity)
        .order(java.nio.ByteOrder.LITTLE_ENDIAN)
    private var position = 0

    fun reset(requiredCapacity: Int) {
        if (buffer.capacity() < requiredCapacity) {
            buffer = java.nio.ByteBuffer
                .allocateDirect(requiredCapacity)
                .order(java.nio.ByteOrder.LITTLE_ENDIAN)
        }
        position = 0
    }

    fun toByteArray(): ByteArray {
        val bytes = ByteArray(position)
        val view = buffer.duplicate()
        view.position(0)
        view.get(bytes, 0, position)
        return bytes
    }

    fun writeBool(value: Boolean) {
        ensureCapacity(1)
        buffer.put(position, if (value) 1.toByte() else 0.toByte())
        position += 1
    }

    fun writeI8(value: Byte) {
        ensureCapacity(1)
        buffer.put(position, value)
        position += 1
    }

    fun writeU8(value: UByte) {
        writeI8(value.toByte())
    }

    fun writeI16(value: Short) {
        ensureCapacity(2)
        buffer.putShort(position, value)
        position += 2
    }

    fun writeU16(value: UShort) {
        writeI16(value.toShort())
    }

    fun writeI32(value: Int) {
        ensureCapacity(4)
        buffer.putInt(position, value)
        position += 4
    }

    fun writeU32(value: UInt) {
        writeI32(value.toInt())
    }

    fun writeI64(value: Long) {
        ensureCapacity(8)
        buffer.putLong(position, value)
        position += 8
    }

    fun writeU64(value: ULong) {
        writeI64(value.toLong())
    }

    fun writeF32(value: Float) {
        writeI32(java.lang.Float.floatToRawIntBits(value))
    }

    fun writeF64(value: Double) {
        writeI64(java.lang.Double.doubleToRawLongBits(value))
    }

    fun writeOptionalBool(value: Boolean?) = writeOptional(value) { writer, present ->
        writer.writeBool(present)
    }

    fun writeOptionalI8(value: Byte?) = writeOptional(value) { writer, present ->
        writer.writeI8(present)
    }

    fun writeOptionalU8(value: UByte?) = writeOptional(value) { writer, present ->
        writer.writeU8(present)
    }

    fun writeOptionalI16(value: Short?) = writeOptional(value) { writer, present ->
        writer.writeI16(present)
    }

    fun writeOptionalU16(value: UShort?) = writeOptional(value) { writer, present ->
        writer.writeU16(present)
    }

    fun writeOptionalI32(value: Int?) = writeOptional(value) { writer, present ->
        writer.writeI32(present)
    }

    fun writeOptionalU32(value: UInt?) = writeOptional(value) { writer, present ->
        writer.writeU32(present)
    }

    fun writeOptionalI64(value: Long?) = writeOptional(value) { writer, present ->
        writer.writeI64(present)
    }

    fun writeOptionalU64(value: ULong?) = writeOptional(value) { writer, present ->
        writer.writeU64(present)
    }

    fun writeOptionalF32(value: Float?) = writeOptional(value) { writer, present ->
        writer.writeF32(present)
    }

    fun writeOptionalF64(value: Double?) = writeOptional(value) { writer, present ->
        writer.writeF64(present)
    }

    fun writeString(value: String) {
        val bytes = value.toByteArray(Charsets.UTF_8)
        writeU32(bytes.size.toUInt())
        writeBytesRaw(bytes)
    }

    fun writeBytes(value: ByteArray) {
        writeU32(value.size.toUInt())
        writeBytesRaw(value)
    }

    private fun writeBytesRaw(bytes: ByteArray) {
        ensureCapacity(bytes.size)
        val view = buffer.duplicate().order(java.nio.ByteOrder.LITTLE_ENDIAN)
        view.position(position)
        view.put(bytes)
        position += bytes.size
    }

    private fun ensureCapacity(needed: Int) {
        val required = position + needed
        if (required <= buffer.capacity()) {
            return
        }
        val nextCapacity = maxOf(buffer.capacity() * 2, required)
        val next = java.nio.ByteBuffer
            .allocateDirect(nextCapacity)
            .order(java.nio.ByteOrder.LITTLE_ENDIAN)
        val source = buffer.duplicate().order(java.nio.ByteOrder.LITTLE_ENDIAN)
        source.limit(position)
        source.position(0)
        next.put(source)
        buffer = next
    }

    private inline fun <T> writeOptional(value: T?, write: (WireWriter, T) -> Unit) {
        if (value == null) {
            writeU8(0.toUByte())
            return
        }
        writeU8(1.toUByte())
        write(this, value)
    }
}

private const val MAX_CACHED_WIRE_WRITER_BYTES: Int = 1024 * 1024

private class WireWriterPoolState(private val cacheSize: Int = 4) {
    private val cachedWriters: Array<WireWriter?> = arrayOfNulls(cacheSize)
    private var depth = 0

    fun acquire(requiredCapacity: Int): BorrowedWireWriter {
        val slot = depth
        depth = slot + 1
        val shouldCache = requiredCapacity <= MAX_CACHED_WIRE_WRITER_BYTES && slot < cacheSize
        val writer = if (shouldCache) {
            cachedWriters[slot] ?: WireWriter(requiredCapacity).also { cachedWriters[slot] = it }
        } else {
            WireWriter(requiredCapacity)
        }

        writer.reset(requiredCapacity)
        return BorrowedWireWriter(this, writer)
    }

    fun release() {
        depth -= 1
    }
}

private class BorrowedWireWriter(
    private val state: WireWriterPoolState,
    val writer: WireWriter,
) : AutoCloseable {
    fun bytes(): ByteArray = writer.toByteArray()

    override fun close() {
        state.release()
    }
}

private object WireWriterPool {
    private val state: ThreadLocal<WireWriterPoolState> =
        ThreadLocal.withInitial { WireWriterPoolState() }

    fun acquire(requiredCapacity: Int): BorrowedWireWriter {
        val poolState = state.get() ?: WireWriterPoolState().also { state.set(it) }
        return poolState.acquire(requiredCapacity)
    }
}
