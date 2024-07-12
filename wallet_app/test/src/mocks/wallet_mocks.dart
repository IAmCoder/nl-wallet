import 'package:flutter/widgets.dart';
import 'package:get_it/get_it.dart';
import 'package:mockito/annotations.dart';
import 'package:mockito/mockito.dart';
import 'package:wallet/src/data/repository/card/wallet_card_repository.dart';
import 'package:wallet/src/data/repository/configuration/configuration_repository.dart';
import 'package:wallet/src/data/repository/event/wallet_event_repository.dart';
import 'package:wallet/src/data/repository/language/language_repository.dart';
import 'package:wallet/src/data/repository/pid/pid_repository.dart';
import 'package:wallet/src/data/repository/wallet/wallet_repository.dart';
import 'package:wallet/src/data/service/app_lifecycle_service.dart';
import 'package:wallet/src/data/service/navigation_service.dart';
import 'package:wallet/src/domain/usecase/app/check_is_app_initialized_usecase.dart';
import 'package:wallet/src/domain/usecase/card/get_wallet_card_usecase.dart';
import 'package:wallet/src/domain/usecase/card/get_wallet_cards_usecase.dart';
import 'package:wallet/src/domain/usecase/card/observe_wallet_card_detail_usecase.dart';
import 'package:wallet/src/domain/usecase/card/observe_wallet_card_usecase.dart';
import 'package:wallet/src/domain/usecase/card/observe_wallet_cards_usecase.dart';
import 'package:wallet/src/domain/usecase/disclosure/accept_disclosure_usecase.dart';
import 'package:wallet/src/domain/usecase/disclosure/cancel_disclosure_usecase.dart';
import 'package:wallet/src/domain/usecase/disclosure/start_disclosure_usecase.dart';
import 'package:wallet/src/domain/usecase/event/get_wallet_events_for_card_usecase.dart';
import 'package:wallet/src/domain/usecase/history/observe_recent_history_usecase.dart';
import 'package:wallet/src/domain/usecase/issuance/accept_issuance_usecase.dart';
import 'package:wallet/src/domain/usecase/navigation/check_navigation_prerequisites_usecase.dart';
import 'package:wallet/src/domain/usecase/navigation/perform_pre_navigation_actions_usecase.dart';
import 'package:wallet/src/domain/usecase/network/check_has_internet_usecase.dart';
import 'package:wallet/src/domain/usecase/pid/accept_offered_pid_usecase.dart';
import 'package:wallet/src/domain/usecase/pid/cancel_pid_issuance_usecase.dart';
import 'package:wallet/src/domain/usecase/pid/continue_pid_issuance_usecase.dart';
import 'package:wallet/src/domain/usecase/pid/get_pid_issuance_url_usecase.dart';
import 'package:wallet/src/domain/usecase/pin/check_is_valid_pin_usecase.dart';
import 'package:wallet/src/domain/usecase/pin/unlock_wallet_with_pin_usecase.dart';
import 'package:wallet/src/domain/usecase/sign/accept_sign_agreement_usecase.dart';
import 'package:wallet/src/domain/usecase/sign/reject_sign_agreement_usecase.dart';
import 'package:wallet/src/domain/usecase/sign/start_sign_usecase.dart';
import 'package:wallet/src/domain/usecase/uri/decode_uri_usecase.dart';
import 'package:wallet/src/domain/usecase/wallet/create_wallet_usecase.dart';
import 'package:wallet/src/domain/usecase/wallet/is_wallet_initialized_with_pid_usecase.dart';
import 'package:wallet/src/domain/usecase/wallet/observe_wallet_locked_usecase.dart';
import 'package:wallet/src/domain/usecase/wallet/reset_wallet_usecase.dart';
import 'package:wallet/src/domain/usecase/wallet/setup_mocked_wallet_usecase.dart';
import 'package:wallet/src/util/extension/bloc_extension.dart';
import 'package:wallet/src/util/mapper/mapper.dart';
import 'package:wallet/src/wallet_core/typed/typed_wallet_core.dart';
import 'package:wallet_core/core.dart';

import 'wallet_mocks.mocks.dart';

export 'wallet_mocks.mocks.dart';

/// Mock framework
@GenerateNiceMocks([MockSpec<NavigatorState>()])

/// Mock mappers
@GenerateNiceMocks([MockSpec<Mapper>()])

/// Mock repositories
@GenerateNiceMocks([MockSpec<PidRepository>()])
@GenerateNiceMocks([MockSpec<DisclosureRepository>()])
@GenerateNiceMocks([MockSpec<WalletRepository>()])
@GenerateNiceMocks([MockSpec<WalletCardRepository>()])
@GenerateNiceMocks([MockSpec<WalletEventRepository>()])
@GenerateNiceMocks([MockSpec<ConfigurationRepository>()])
@GenerateNiceMocks([MockSpec<LanguageRepository>()])

/// Mock services
@GenerateNiceMocks([MockSpec<TypedWalletCore>()])
@GenerateNiceMocks([MockSpec<NavigationService>()])

/// Mock use cases
@GenerateNiceMocks([MockSpec<DecodeUriUseCase>()])
@GenerateNiceMocks([MockSpec<IsWalletInitializedUseCase>()])
@GenerateNiceMocks([MockSpec<IsWalletInitializedWithPidUseCase>()])
@GenerateNiceMocks([MockSpec<ObserveWalletLockedUseCase>()])
@GenerateNiceMocks([MockSpec<CheckPinUseCase>()])
@GenerateNiceMocks([MockSpec<SetupMockedWalletUseCase>()])
@GenerateNiceMocks([MockSpec<CheckHasInternetUseCase>()])
@GenerateNiceMocks([MockSpec<PerformPreNavigationActionsUseCase>()])
@GenerateNiceMocks([MockSpec<CheckNavigationPrerequisitesUseCase>()])
@GenerateNiceMocks([MockSpec<AcceptOfferedPidUseCase>()])
@GenerateNiceMocks([MockSpec<AcceptSignAgreementUseCase>()])
@GenerateNiceMocks([MockSpec<AcceptDisclosureUseCase>()])
@GenerateNiceMocks([MockSpec<AcceptIssuanceUseCase>()])
@GenerateNiceMocks([MockSpec<StartDisclosureUseCase>()])
@GenerateNiceMocks([MockSpec<CancelDisclosureUseCase>()])
@GenerateNiceMocks([MockSpec<ObserveWalletCardsUseCase>()])
@GenerateNiceMocks([MockSpec<ObserveRecentHistoryUseCase>()])
@GenerateNiceMocks([MockSpec<CheckIsValidPinUseCase>()])
@GenerateNiceMocks([MockSpec<CreateWalletUseCase>()])
@GenerateNiceMocks([MockSpec<UnlockWalletWithPinUseCase>()])
@GenerateNiceMocks([MockSpec<ResetWalletUseCase>()])
@GenerateNiceMocks([MockSpec<ObserveWalletCardUseCase>()])
@GenerateNiceMocks([MockSpec<GetWalletCardsUseCase>()])
@GenerateNiceMocks([MockSpec<GetPidIssuanceUrlUseCase>()])
@GenerateNiceMocks([MockSpec<CancelPidIssuanceUseCase>()])
@GenerateNiceMocks([MockSpec<ContinuePidIssuanceUseCase>()])
@GenerateNiceMocks([MockSpec<ObserveWalletCardDetailUseCase>()])
@GenerateNiceMocks([MockSpec<GetWalletCardUseCase>()])
@GenerateNiceMocks([MockSpec<GetWalletEventsForCardUseCase>()])
@GenerateNiceMocks([MockSpec<StartSignUseCase>()])
@GenerateNiceMocks([MockSpec<RejectSignAgreementUseCase>()])

/// Core
@GenerateNiceMocks([MockSpec<WalletCore>()])

/// Constants
const kMockPidIssuanceUrl = 'https://example.org';

/// Class that provides the generated mocks with very
/// basic, or no stubbing. Stubs can be overwritten or the mocks
/// can always be instantiated directly. The main intention here is
/// to allow us to instantiate classes under tests in a simple way,
/// i.e. `xxRepository(Mocks.create(), Mocks.create(), Mocks.create())`
/// When you need more control over what a mock returns you should
/// probably instantiate the mock directly in your test class.
class Mocks {
  Mocks._();

  static final sl = GetIt.asNewInstance();
  static var isInitialized = false;

  static void initialize() {
    // Core
    sl.registerFactory<WalletCore>(MockWalletCore.new);

    // Services
    sl.registerFactory<AppLifecycleService>(AppLifecycleService.new);
    sl.registerFactory<TypedWalletCore>(getTypedWalletCoreMock);

    // Use cases
    sl.registerFactory<DecodeUriUseCase>(MockDecodeUriUseCase.new);
    sl.registerFactory<IsWalletInitializedUseCase>(MockIsWalletInitializedUseCase.new);
    sl.registerFactory<IsWalletInitializedWithPidUseCase>(MockIsWalletInitializedWithPidUseCase.new);
    sl.registerFactory<ObserveWalletLockedUseCase>(MockObserveWalletLockedUseCase.new);
    sl.registerFactory<CheckPinUseCase>(MockCheckPinUseCase.new);
    sl.registerFactory<SetupMockedWalletUseCase>(MockSetupMockedWalletUseCase.new);
    sl.registerFactory<CheckHasInternetUseCase>(() {
      final mock = MockCheckHasInternetUseCase();
      when(mock.invoke()).thenAnswer((realInvocation) async => true);
      BlocExtensions.checkHasInternetUseCase = mock;
      return mock;
    });
    sl.registerFactory<AcceptOfferedPidUseCase>(MockAcceptOfferedPidUseCase.new);
    sl.registerFactory<StartDisclosureUseCase>(MockStartDisclosureUseCase.new);
    sl.registerFactory<CancelDisclosureUseCase>(MockCancelDisclosureUseCase.new);

    // Repositories
    sl.registerFactory<PidRepository>(getMockPidRepository);
    sl.registerFactory<WalletRepository>(MockWalletRepository.new);

    // Mappers
    sl.registerFactory<Mapper>(MockMapper.new);

    isInitialized = true;
  }

  static MockTypedWalletCore getTypedWalletCoreMock() {
    final mock = MockTypedWalletCore();
    when(mock.createPidIssuanceRedirectUri()).thenAnswer((_) async => kMockPidIssuanceUrl);
    return mock;
  }

  static MockPidRepository getMockPidRepository() {
    final mock = MockPidRepository();
    when(mock.getPidIssuanceUrl()).thenAnswer((_) async => kMockPidIssuanceUrl);
    return mock;
  }

  static T create<T extends Object>() {
    if (!isInitialized) initialize();
    return sl.get<T>();
  }
}
