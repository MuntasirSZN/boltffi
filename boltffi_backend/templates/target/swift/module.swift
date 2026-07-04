import Foundation
import {{ module }}

public struct FfiError: Error {
    public let message: String

    public init(message: String) {
        self.message = message
    }
}

