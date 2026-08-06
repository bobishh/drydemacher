import Foundation
import RealityKit

enum CaptureFailure: Error, CustomStringConvertible {
    case usage
    case unsupported
    case request(String)
    case noModel

    var description: String {
        switch self {
        case .usage: return "usage: ecky-capture-object <input-folder> <output-folder>"
        case .unsupported: return "Apple Object Capture is unavailable on this Mac."
        case .request(let message): return message
        case .noModel: return "Apple Object Capture completed without an OBJ model."
        }
    }
}

@main
struct EckyCaptureObject {
    static func main() async {
        do {
            if CommandLine.arguments.count == 2 && CommandLine.arguments[1] == "--check" {
                guard PhotogrammetrySession.isSupported else { throw CaptureFailure.unsupported }
                print("available")
                return
            }
            guard CommandLine.arguments.count == 3 else { throw CaptureFailure.usage }
            guard PhotogrammetrySession.isSupported else { throw CaptureFailure.unsupported }
            let input = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
            let output = URL(fileURLWithPath: CommandLine.arguments[2], isDirectory: true)
            try? FileManager.default.removeItem(at: output)
            try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)

            let session = try PhotogrammetrySession(input: input)
            let request = PhotogrammetrySession.Request.modelFile(url: output, detail: .medium)
            try session.process(requests: [request])
            var producedModel = false
            for try await event in session.outputs {
                switch event {
                case .requestProgress(_, let fraction):
                    FileHandle.standardError.write(Data("PROGRESS \(fraction)\n".utf8))
                case .requestComplete(_, let result):
                    if case .modelFile(let url) = result {
                        producedModel = true
                        print(url.path)
                    }
                case .requestError(_, let error):
                    throw CaptureFailure.request(error.localizedDescription)
                case .processingComplete:
                    if !producedModel { throw CaptureFailure.noModel }
                    return
                default:
                    continue
                }
            }
            if !producedModel { throw CaptureFailure.noModel }
        } catch {
            FileHandle.standardError.write(Data("\(error)\n".utf8))
            Foundation.exit(1)
        }
    }
}
