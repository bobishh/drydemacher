export type LatestTaskToken = Readonly<{
  target: string;
  revision: number;
}>;

export class LatestTaskGate {
  private nextRevision = 0;
  private readonly latestByTarget = new Map<string, number>();

  reserve(target: string): LatestTaskToken {
    const revision = ++this.nextRevision;
    this.latestByTarget.set(target, revision);
    return { target, revision };
  }

  isCurrent(token: LatestTaskToken): boolean {
    return this.latestByTarget.get(token.target) === token.revision;
  }
}
