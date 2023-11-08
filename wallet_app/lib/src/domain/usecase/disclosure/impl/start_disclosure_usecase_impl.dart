import '../start_disclosure_usecase.dart';

class StartDisclosureUseCaseImpl extends StartDisclosureUseCase {
  final DisclosureRepository _disclosureRepository;

  StartDisclosureUseCaseImpl(this._disclosureRepository);

  @override
  Future<StartDisclosureResult> invoke(String disclosureUri) => _disclosureRepository.startDisclosure(disclosureUri);
}
