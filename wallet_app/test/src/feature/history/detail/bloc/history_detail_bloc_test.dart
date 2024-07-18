import 'package:bloc_test/bloc_test.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mockito/mockito.dart';
import 'package:wallet/src/feature/history/detail/bloc/history_detail_bloc.dart';

import '../../../../mocks/wallet_mock_data.dart';
import '../../../../mocks/wallet_mocks.mocks.dart';

void main() {
  late MockGetWalletCardsUseCase getWalletCardsUseCase;

  setUp(() {
    getWalletCardsUseCase = MockGetWalletCardsUseCase();
  });

  blocTest(
    'verify initial state',
    build: () => HistoryDetailBloc(getWalletCardsUseCase),
    verify: (bloc) => expect(bloc.state, HistoryDetailInitial()),
  );

  blocTest(
    'verify transition to HistoryDetailLoadFailure when cards can not be loaded',
    build: () => HistoryDetailBloc(getWalletCardsUseCase),
    setUp: () => when(getWalletCardsUseCase.invoke()).thenAnswer((_) => Future.error('Could not load cards')),
    act: (bloc) => bloc.add(HistoryDetailLoadTriggered(event: WalletMockData.disclosureEvent)),
    expect: () => [const HistoryDetailLoadInProgress(), const HistoryDetailLoadFailure()],
  );

  blocTest(
    'verify transition to HistoryDetailLoadSuccess when cards can be loaded',
    build: () => HistoryDetailBloc(getWalletCardsUseCase),
    setUp: () =>
        when(getWalletCardsUseCase.invoke()).thenAnswer((_) async => [WalletMockData.card, WalletMockData.altCard]),
    act: (bloc) => bloc.add(HistoryDetailLoadTriggered(event: WalletMockData.disclosureEvent)),
    expect: () => [
      const HistoryDetailLoadInProgress(),
      HistoryDetailLoadSuccess(WalletMockData.disclosureEvent, [WalletMockData.card]),
    ],
  );
}
