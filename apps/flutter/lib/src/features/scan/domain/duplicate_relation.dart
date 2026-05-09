enum DuplicateRelationKind {
  sameNameAndSize('SAME_NAME_AND_SIZE'),
  sameSizeAndHash('SAME_SIZE_AND_HASH');

  const DuplicateRelationKind(this.id);

  final String id;
}

class DuplicateRelation {
  const DuplicateRelation({
    required this.originalPath,
    required this.duplicatePath,
    required this.currentDuplicatePath,
    required this.size,
    required this.hash,
    required this.kind,
  });

  final String originalPath;
  final String duplicatePath;
  final String currentDuplicatePath;
  final int size;
  final String hash;
  final DuplicateRelationKind kind;
}
