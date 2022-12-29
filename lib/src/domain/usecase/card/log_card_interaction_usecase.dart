import '../../../data/repository/card/timeline_attribute_repository.dart';
import '../../../feature/verification/model/organization.dart';
import '../../model/attribute/data_attribute.dart';
import '../../model/policy/policy.dart';
import '../../model/timeline/timeline_attribute.dart';

class LogCardInteractionUseCase {
  final TimelineAttributeRepository timelineAttributeRepository;

  LogCardInteractionUseCase(this.timelineAttributeRepository);

  Future<void> invoke(
    InteractionStatus status,
    Policy policy,
    Organization organization,
    String cardId,
    List<DataAttribute> attributes,
  ) async {
    final interaction = InteractionAttribute(
      status: status,
      policy: policy,
      dateTime: DateTime.now(),
      organization: organization,
      attributes: _getFilteredAttributes(status, attributes),
    );
    await timelineAttributeRepository.create(cardId, interaction);
  }

  /// Filters attributes for storage; only returns attributes for 'success' interaction
  List<DataAttribute> _getFilteredAttributes(InteractionStatus status, List<DataAttribute> attributes) {
    if (status == InteractionStatus.success) return attributes;
    return [];
  }
}
